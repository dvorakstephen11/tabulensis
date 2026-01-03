import assert from "node:assert/strict";
import { buildWorkbookViewModel } from "./view_model.js";

function findSheet(vm, name) {
  const sheet = vm.sheets.find(item => item.name === name);
  assert.ok(sheet, `Missing sheet ${name}`);
  return sheet;
}

function makeCellEdited(sheetId, row, col) {
  return {
    kind: "CellEdited",
    sheet: sheetId,
    addr: { row, col },
    from: { value: { Number: row } },
    to: { value: { Number: row + 1 } }
  };
}

function testRowInsertionMapping() {
  const report = {
    strings: ["Sheet1"],
    ops: [{ kind: "RowAdded", sheet: 0, row_idx: 0 }],
    warnings: []
  };
  const oldSheet = {
    name: "Sheet1",
    nrows: 2,
    ncols: 1,
    cells: [
      { row: 0, col: 0, value: "A", formula: null },
      { row: 1, col: 0, value: "B", formula: null }
    ]
  };
  const newSheet = {
    name: "Sheet1",
    nrows: 3,
    ncols: 1,
    cells: [
      { row: 0, col: 0, value: "X", formula: null },
      { row: 1, col: 0, value: "A", formula: null },
      { row: 2, col: 0, value: "B", formula: null }
    ]
  };
  const alignment = {
    sheet: "Sheet1",
    rows: [
      { old: null, new: 0, kind: "insert" },
      { old: 0, new: 1, kind: "match" },
      { old: 1, new: 2, kind: "match" }
    ],
    cols: [{ old: 0, new: 0, kind: "match" }],
    moves: [],
    skipped: false
  };
  const payload = {
    report,
    sheets: { old: { sheets: [oldSheet] }, new: { sheets: [newSheet] } },
    alignments: [alignment]
  };
  const vm = buildWorkbookViewModel(payload);
  const sheetVm = findSheet(vm, "Sheet1");
  const cell = sheetVm.cellAt(1, 0);
  assert.equal(cell.diffKind, "unchanged");
  assert.equal(cell.old.row, 0);
  assert.equal(cell.new.row, 1);
}

function testMoveIdentity() {
  const report = {
    strings: ["MoveSheet"],
    ops: [
      { kind: "BlockMovedRows", sheet: 0, src_start_row: 0, dst_start_row: 1, row_count: 1 }
    ],
    warnings: []
  };
  const oldSheet = {
    name: "MoveSheet",
    nrows: 2,
    ncols: 1,
    cells: [
      { row: 0, col: 0, value: "A", formula: null },
      { row: 1, col: 0, value: "B", formula: null }
    ]
  };
  const newSheet = {
    name: "MoveSheet",
    nrows: 2,
    ncols: 1,
    cells: [
      { row: 0, col: 0, value: "B", formula: null },
      { row: 1, col: 0, value: "A", formula: null }
    ]
  };
  const alignment = {
    sheet: "MoveSheet",
    rows: [
      { old: 0, new: null, kind: "move_src", move_id: "r:0+1->1" },
      { old: 1, new: 0, kind: "match" },
      { old: null, new: 1, kind: "move_dst", move_id: "r:0+1->1" }
    ],
    cols: [{ old: 0, new: 0, kind: "match" }],
    moves: [{ id: "r:0+1->1", axis: "row", src_start: 0, dst_start: 1, count: 1 }],
    skipped: false
  };
  const payload = {
    report,
    sheets: { old: { sheets: [oldSheet] }, new: { sheets: [newSheet] } },
    alignments: [alignment]
  };
  const vm = buildWorkbookViewModel(payload);
  const sheetVm = findSheet(vm, "MoveSheet");

  const srcCell = sheetVm.cellAt(0, 0);
  assert.equal(srcCell.diffKind, "moved");
  assert.equal(srcCell.moveRole, "src");
  assert.equal(srcCell.moveId, "r:0+1->1");

  const dstCell = sheetVm.cellAt(2, 0);
  assert.equal(dstCell.diffKind, "moved");
  assert.equal(dstCell.moveRole, "dst");
  assert.equal(dstCell.moveId, "r:0+1->1");
}

function testRowGrouping() {
  const report = { strings: ["GroupSheet"], ops: [], warnings: [] };
  const ops = [];
  for (let i = 0; i < 10; i++) {
    ops.push({ kind: "RowAdded", sheet: 0, row_idx: i });
  }
  report.ops = ops;

  const oldSheet = { name: "GroupSheet", nrows: 0, ncols: 1, cells: [] };
  const newSheet = { name: "GroupSheet", nrows: 10, ncols: 1, cells: [] };
  const rows = [];
  for (let i = 0; i < 10; i++) {
    rows.push({ old: null, new: i, kind: "insert" });
  }
  const alignment = {
    sheet: "GroupSheet",
    rows,
    cols: [{ old: 0, new: 0, kind: "match" }],
    moves: [],
    skipped: false
  };
  const payload = {
    report,
    sheets: { old: { sheets: [oldSheet] }, new: { sheets: [newSheet] } },
    alignments: [alignment]
  };

  const vm = buildWorkbookViewModel(payload);
  const sheetVm = findSheet(vm, "GroupSheet");
  const rowAdds = sheetVm.changes.items.filter(item => item.group === "rows" && item.changeType === "added");
  assert.equal(rowAdds.length, 1);
  assert.ok(/Rows 1-10 added/.test(rowAdds[0].label));
}

function testRegionCompaction() {
  const report = { strings: ["RegionSheet"], ops: [], warnings: [] };
  const ops = [
    makeCellEdited(0, 0, 0),
    makeCellEdited(0, 0, 1),
    makeCellEdited(0, 1, 0),
    makeCellEdited(0, 1, 1),
    makeCellEdited(0, 100, 0),
    makeCellEdited(0, 100, 1),
    makeCellEdited(0, 101, 0),
    makeCellEdited(0, 101, 1)
  ];
  report.ops = ops;

  const oldSheet = { name: "RegionSheet", nrows: 120, ncols: 2, cells: [] };
  const newSheet = { name: "RegionSheet", nrows: 120, ncols: 2, cells: [] };
  const rows = [];
  for (let i = 0; i < 120; i++) {
    rows.push({ old: i, new: i, kind: "match" });
  }
  const cols = [
    { old: 0, new: 0, kind: "match" },
    { old: 1, new: 1, kind: "match" }
  ];
  const alignment = {
    sheet: "RegionSheet",
    rows,
    cols,
    moves: [],
    skipped: false
  };
  const payload = {
    report,
    sheets: { old: { sheets: [oldSheet] }, new: { sheets: [newSheet] } },
    alignments: [alignment]
  };
  const vm = buildWorkbookViewModel(payload, { maxCellsPerRegion: 100 });
  const sheetVm = findSheet(vm, "RegionSheet");
  const regions = sheetVm.changes.regions.filter(region => region.kind === "cell");
  assert.equal(regions.length, 2);
}

function testRegionMaxCells() {
  const report = { strings: ["CapSheet"], ops: [], warnings: [] };
  const ops = [];
  for (let col = 0; col < 120; col++) {
    ops.push(makeCellEdited(0, 0, col));
  }
  report.ops = ops;

  const oldSheet = { name: "CapSheet", nrows: 1, ncols: 120, cells: [] };
  const newSheet = { name: "CapSheet", nrows: 1, ncols: 120, cells: [] };
  const rows = [{ old: 0, new: 0, kind: "match" }];
  const cols = [];
  for (let col = 0; col < 120; col++) {
    cols.push({ old: col, new: col, kind: "match" });
  }
  const alignment = {
    sheet: "CapSheet",
    rows,
    cols,
    moves: [],
    skipped: false
  };
  const payload = {
    report,
    sheets: { old: { sheets: [oldSheet] }, new: { sheets: [newSheet] } },
    alignments: [alignment]
  };
  const vm = buildWorkbookViewModel(payload, { maxCellsPerRegion: 50 });
  const sheetVm = findSheet(vm, "CapSheet");
  const regions = sheetVm.changes.regions.filter(region => region.kind === "cell");
  assert.ok(regions.length >= 3);
  for (const region of regions) {
    assert.ok(region.cellCount <= 50);
  }
}

function testAnchorsForRowChanges() {
  const report = {
    strings: ["AnchorSheet"],
    ops: [{ kind: "RowAdded", sheet: 0, row_idx: 0 }],
    warnings: []
  };
  const oldSheet = { name: "AnchorSheet", nrows: 1, ncols: 1, cells: [] };
  const newSheet = { name: "AnchorSheet", nrows: 2, ncols: 1, cells: [] };
  const alignment = {
    sheet: "AnchorSheet",
    rows: [
      { old: null, new: 0, kind: "insert" },
      { old: 0, new: 1, kind: "match" }
    ],
    cols: [{ old: 0, new: 0, kind: "match" }],
    moves: [],
    skipped: false
  };
  const payload = {
    report,
    sheets: { old: { sheets: [oldSheet] }, new: { sheets: [newSheet] } },
    alignments: [alignment]
  };
  const vm = buildWorkbookViewModel(payload);
  const sheetVm = findSheet(vm, "AnchorSheet");
  const anchor = sheetVm.changes.anchors.find(item => item.id === "row:added:0-0");
  assert.ok(anchor, "Missing row add anchor");
  assert.equal(anchor.target.kind, "grid");
  assert.equal(anchor.target.viewRow, 0);
}

function testIgnoreBlankToBlank() {
  const report = {
    strings: ["BlankSheet"],
    ops: [
      {
        kind: "CellEdited",
        sheet: 0,
        addr: { row: 0, col: 0 },
        from: { value: "Blank" },
        to: { value: "Blank" }
      }
    ],
    warnings: []
  };
  const oldSheet = { name: "BlankSheet", nrows: 1, ncols: 1, cells: [] };
  const newSheet = { name: "BlankSheet", nrows: 1, ncols: 1, cells: [] };
  const alignment = {
    sheet: "BlankSheet",
    rows: [{ old: 0, new: 0, kind: "match" }],
    cols: [{ old: 0, new: 0, kind: "match" }],
    moves: [],
    skipped: false
  };
  const payload = {
    report,
    sheets: { old: { sheets: [oldSheet] }, new: { sheets: [newSheet] } },
    alignments: [alignment]
  };

  const vmDefault = buildWorkbookViewModel(payload);
  const sheetDefault = findSheet(vmDefault, "BlankSheet");
  assert.equal(sheetDefault.changes.items.length, 0);
  assert.equal(sheetDefault.changes.regions.length, 0);
  assert.equal(sheetDefault.changes.anchors.length, 0);

  const vmInclude = buildWorkbookViewModel(payload, { ignoreBlankToBlank: false });
  const sheetInclude = findSheet(vmInclude, "BlankSheet");
  assert.ok(sheetInclude.changes.regions.length > 0);
  assert.ok(sheetInclude.changes.anchors.length > 0);
}

function testSheetRenameMapping() {
  const report = {
    strings: ["OldSheet", "NewSheet"],
    ops: [
      { kind: "SheetRenamed", sheet: 1, from: 0, to: 1 }
    ],
    warnings: []
  };
  const oldSheet = {
    name: "OldSheet",
    nrows: 1,
    ncols: 1,
    cells: [{ row: 0, col: 0, value: "A", formula: null }]
  };
  const newSheet = {
    name: "NewSheet",
    nrows: 1,
    ncols: 1,
    cells: [{ row: 0, col: 0, value: "B", formula: null }]
  };
  const alignment = {
    sheet: "NewSheet",
    rows: [{ old: 0, new: 0, kind: "match" }],
    cols: [{ old: 0, new: 0, kind: "match" }],
    moves: [],
    skipped: false
  };
  const payload = {
    report,
    sheets: { old: { sheets: [oldSheet] }, new: { sheets: [newSheet] } },
    alignments: [alignment]
  };
  const vm = buildWorkbookViewModel(payload);
  const sheetVm = findSheet(vm, "NewSheet");
  const cell = sheetVm.cellAt(0, 0);
  assert.equal(cell.old.cell.value, "A");
  assert.equal(cell.new.cell.value, "B");
}

testRowInsertionMapping();
testMoveIdentity();
testRowGrouping();
testRegionCompaction();
testRegionMaxCells();
testAnchorsForRowChanges();
testIgnoreBlankToBlank();
testSheetRenameMapping();

console.log("ok");
