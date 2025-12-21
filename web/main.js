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

document.addEventListener('DOMContentLoaded', async () => {
    setupFileInput('oldFile', 'oldFileBox', 'oldFileName', (data) => { oldFileData = data; });
    setupFileInput('newFile', 'newFileBox', 'newFileName', (data) => { newFileData = data; });

    document.getElementById('diffBtn').addEventListener('click', runDiff);
    document.getElementById('downloadBtn').addEventListener('click', downloadJson);

    await initWasm();
    updateDiffButton();
});

