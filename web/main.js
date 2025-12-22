import init, { diff_workbooks_json, diff_summary, get_version } from './wasm/excel_diff_wasm.js';

let wasmReady = false;
let oldFileData = null;
let newFileData = null;
let lastReport = null;

async function initWasm() {
    try {
        await init();
        wasmReady = true;
        console.log('Excel Diff WASM initialized, version:', get_version());
    } catch (err) {
        showError('Failed to initialize WebAssembly module: ' + err.message);
    }
}

function showError(message) {
    const errorEl = document.getElementById('error');
    errorEl.textContent = message;
    errorEl.style.display = 'block';
}

function hideError() {
    document.getElementById('error').style.display = 'none';
}

function updateDiffButton() {
    const btn = document.getElementById('diffBtn');
    btn.disabled = !wasmReady || !oldFileData || !newFileData;
}

function setupFileInput(inputId, boxId, nameId, setter) {
    const input = document.getElementById(inputId);
    const box = document.getElementById(boxId);
    const nameEl = document.getElementById(nameId);

    input.addEventListener('change', async (e) => {
        const file = e.target.files[0];
        if (!file) return;

        try {
            const buffer = await file.arrayBuffer();
            setter(new Uint8Array(buffer));
            nameEl.textContent = file.name;
            box.classList.add('has-file');
            hideError();
            updateDiffButton();
        } catch (err) {
            showError('Failed to read file: ' + err.message);
        }
    });

    box.addEventListener('dragover', (e) => {
        e.preventDefault();
        box.classList.add('dragover');
    });

    box.addEventListener('dragleave', () => {
        box.classList.remove('dragover');
    });

    box.addEventListener('drop', async (e) => {
        e.preventDefault();
        box.classList.remove('dragover');

        const file = e.dataTransfer.files[0];
        if (!file) return;

        if (!file.name.endsWith('.xlsx') && !file.name.endsWith('.xlsm')) {
            showError('Please drop an Excel file (.xlsx or .xlsm)');
            return;
        }

        try {
            const buffer = await file.arrayBuffer();
            setter(new Uint8Array(buffer));
            nameEl.textContent = file.name;
            box.classList.add('has-file');
            hideError();
            updateDiffButton();
        } catch (err) {
            showError('Failed to read file: ' + err.message);
        }
    });
}

async function runDiff() {
    if (!wasmReady || !oldFileData || !newFileData) return;

    const btn = document.getElementById('diffBtn');
    const results = document.getElementById('results');
    const emptyState = document.getElementById('emptyState');

    btn.innerHTML = '<span class="loading-spinner"></span>Comparing...';
    btn.classList.add('loading');
    hideError();

    try {
        const summary = diff_summary(oldFileData, newFileData);
        
        document.getElementById('opCount').textContent = summary.op_count;
        document.getElementById('opCount').className = 'summary-value' + 
            (summary.op_count > 100 ? ' danger' : summary.op_count > 0 ? ' warning' : '');
        document.getElementById('sheetsOld').textContent = summary.sheets_old;
        document.getElementById('sheetsNew').textContent = summary.sheets_new;

        const jsonReport = diff_workbooks_json(oldFileData, newFileData);
        lastReport = jsonReport;

        const parsed = JSON.parse(jsonReport);
        const preview = JSON.stringify(parsed, null, 2);
        const maxPreviewLength = 50000;

        renderQueryDetails(parsed);

        document.getElementById('diffOutput').textContent = 
            preview.length > maxPreviewLength 
                ? preview.substring(0, maxPreviewLength) + '\n\n... (truncated, download full JSON for complete report)'
                : preview;

        results.style.display = 'block';
        emptyState.style.display = 'none';
    } catch (err) {
        showError('Diff failed: ' + err);
        results.style.display = 'none';
        emptyState.style.display = 'block';
    } finally {
        btn.innerHTML = 'Compare Workbooks';
        btn.classList.remove('loading');
    }
}

function downloadJson() {
    if (!lastReport) return;

    const blob = new Blob([lastReport], { type: 'application/json' });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = 'excel-diff-report.json';
    document.body.appendChild(a);
    a.click();
    document.body.removeChild(a);
    URL.revokeObjectURL(url);
}

function renderQueryDetails(report) {
    const container = document.getElementById('queryDetails');
    const body = document.getElementById('queryDetailsBody');
    body.innerHTML = '';

    const ops = (report.ops || []).filter((op) => op.kind === 'QueryDefinitionChanged');
    if (!ops.length) {
        container.style.display = 'none';
        return;
    }

    const strings = report.strings || [];
    for (const op of ops) {
        body.appendChild(renderQueryDefinitionChanged(op, strings));
    }
    container.style.display = 'block';
}

function renderQueryDefinitionChanged(op, strings) {
    const wrapper = document.createElement('div');
    wrapper.className = 'query-op';

    const header = document.createElement('div');
    header.className = 'query-op-header';
    header.textContent = `Query definition changed: ${resolveString(strings, op.name)} (${op.change_kind})`;
    wrapper.appendChild(header);

    const detail = op.semantic_detail;
    if (!detail) return wrapper;

    const summary = document.createElement('div');
    summary.className = 'op-detail-summary';

    if (detail.step_diffs && detail.step_diffs.length) {
        let added = 0, removed = 0, modified = 0, reordered = 0;
        for (const d of detail.step_diffs) {
            if (d.kind === 'step_added') added++;
            else if (d.kind === 'step_removed') removed++;
            else if (d.kind === 'step_modified') modified++;
            else if (d.kind === 'step_reordered') reordered++;
        }
        summary.textContent = `steps: +${added} -${removed} ~${modified} r${reordered}`;
    } else if (detail.ast_summary) {
        const a = detail.ast_summary;
        summary.textContent = `ast: moved=${a.moved} inserted=${a.inserted} deleted=${a.deleted} updated=${a.updated}`;
    }

    const btn = document.createElement('button');
    btn.textContent = 'details';
    btn.className = 'toggle';

    const body = document.createElement('div');
    body.className = 'op-detail-body';
    body.style.display = 'none';

    btn.onclick = () => {
        body.style.display = body.style.display === 'none' ? 'block' : 'none';
    };

    if (detail.step_diffs && detail.step_diffs.length) {
        const ul = document.createElement('ul');
        for (const d of detail.step_diffs) {
            ul.appendChild(renderStepDiff(d, strings));
        }
        body.appendChild(ul);
    } else if (detail.ast_summary && detail.ast_summary.move_hints) {
        const ul = document.createElement('ul');
        for (const mh of detail.ast_summary.move_hints.slice(0, 20)) {
            const li = document.createElement('li');
            li.textContent = `move hash=${mh.subtree_hash} size=${mh.subtree_size} from=${mh.from_preorder} to=${mh.to_preorder}`;
            ul.appendChild(li);
        }
        body.appendChild(ul);
    }

    wrapper.appendChild(summary);
    wrapper.appendChild(btn);
    wrapper.appendChild(body);
    return wrapper;
}

function renderStepDiff(d, strings) {
    const li = document.createElement('li');

    if (d.kind === 'step_added') {
        const s = d.step;
        li.textContent = `+ ${resolveString(strings, s.name)} (${s.step_type})`;
    } else if (d.kind === 'step_removed') {
        const s = d.step;
        li.textContent = `- ${resolveString(strings, s.name)} (${s.step_type})`;
    } else if (d.kind === 'step_reordered') {
        li.textContent = `r ${resolveString(strings, d.name)} ${d.from_index} -> ${d.to_index}`;
    } else if (d.kind === 'step_modified') {
        const a = d.after;
        li.textContent = `~ ${resolveString(strings, a.name)} (${a.step_type})`;
    } else {
        li.textContent = JSON.stringify(d);
    }

    return li;
}

function resolveString(strings, id) {
    if (!Array.isArray(strings)) return '(unknown)';
    return strings[id] || '(unknown)';
}

document.addEventListener('DOMContentLoaded', async () => {
    setupFileInput('oldFile', 'oldFileBox', 'oldFileName', (data) => { oldFileData = data; });
    setupFileInput('newFile', 'newFileBox', 'newFileName', (data) => { newFileData = data; });

    document.getElementById('diffBtn').addEventListener('click', runDiff);
    document.getElementById('downloadBtn').addEventListener('click', downloadJson);

    await initWasm();
    updateDiffButton();
});

