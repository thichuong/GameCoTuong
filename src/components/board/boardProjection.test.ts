import test from 'node:test';
import assert from 'node:assert/strict';
import type { Square } from '../../types/game.js';
import { BOARD_SIZE, getVisualCoords } from './boardCoords.js';
import {
  VIEW_MODE_CAMERAS,
  WOOD_MIN,
  WOOD_MAX,
  isPerspectiveCamera,
  lerpCameraParams,
  projectBoardPoint,
  unprojectToBoard,
  pickBoardSquare,
} from './boardProjection.js';

test('2d camera produces exact identity projection', () => {
  const cam2d = VIEW_MODE_CAMERAS['2d'];
  assert.equal(isPerspectiveCamera(cam2d), false);

  for (let row = 0; row <= 9; row++) {
    for (let col = 0; col <= 8; col++) {
      const { x, y } = getVisualCoords(row, col, 'red');
      const pt = projectBoardPoint(x, y, 0, cam2d);
      assert.ok(Math.abs(pt.x - x) < 1e-9);
      assert.ok(Math.abs(pt.y - y) < 1e-9);
      assert.ok(Math.abs(pt.scale - 1) < 1e-9);
    }
  }
});

test('projectBoardPoint and unprojectToBoard round-trip within 1e-6 for 2.5d, 3d, and mid-lerp', () => {
  const cameras = [
    VIEW_MODE_CAMERAS['2.5d'],
    VIEW_MODE_CAMERAS['3d'],
    lerpCameraParams(VIEW_MODE_CAMERAS['2d'], VIEW_MODE_CAMERAS['3d'], 0.5),
  ];
  const elevations = [0, 17, 35];

  for (const cam of cameras) {
    assert.equal(isPerspectiveCamera(cam), true);
    for (const z of elevations) {
      for (let row = 0; row <= 9; row++) {
        for (let col = 0; col <= 8; col++) {
          const { x: bx, y: by } = getVisualCoords(row, col, 'red');
          const projected = projectBoardPoint(bx, by, z, cam);
          const unprojected = unprojectToBoard(projected.x, projected.y, z, cam);
          assert.ok(unprojected !== null);
          assert.ok(Math.abs(unprojected.x - bx) < 1e-6);
          assert.ok(Math.abs(unprojected.y - by) < 1e-6);
        }
      }
    }
  }
});

test('board slab corners fit cleanly inside 554x554 canvas for 2.5d and 3d', () => {
  for (const mode of ['2.5d', '3d'] as const) {
    const cam = VIEW_MODE_CAMERAS[mode];
    const corners: [number, number, number][] = [
      [WOOD_MIN, WOOD_MIN, 0],
      [WOOD_MAX, WOOD_MIN, 0],
      [WOOD_MIN, WOOD_MAX, 0],
      [WOOD_MAX, WOOD_MAX, 0],
      [WOOD_MIN, WOOD_MAX, -cam.slabThickness],
      [WOOD_MAX, WOOD_MAX, -cam.slabThickness],
    ];

    for (const [bx, by, z] of corners) {
      const pt = projectBoardPoint(bx, by, z, cam);
      assert.ok(
        pt.x >= 0 && pt.x <= BOARD_SIZE,
        `Mode ${mode} X=${pt.x} out of [0, ${BOARD_SIZE}]`
      );
      assert.ok(
        pt.y >= 0 && pt.y <= BOARD_SIZE,
        `Mode ${mode} Y=${pt.y} out of [0, ${BOARD_SIZE}]`
      );
    }

    // Perspective monotonicity: far top row is narrower and smaller scale than near bottom row
    const topLeft = projectBoardPoint(WOOD_MIN, WOOD_MIN, 0, cam);
    const topRight = projectBoardPoint(WOOD_MAX, WOOD_MIN, 0, cam);
    const bottomLeft = projectBoardPoint(WOOD_MIN, WOOD_MAX, 0, cam);
    const bottomRight = projectBoardPoint(WOOD_MAX, WOOD_MAX, 0, cam);
    assert.ok(topRight.x - topLeft.x < bottomRight.x - bottomLeft.x);
    assert.ok(topLeft.scale < bottomLeft.scale);
  }
});

test('pickBoardSquare hits elevated 3D piece top faces and empty intersections for both sides', () => {
  const cam3d = VIEW_MODE_CAMERAS['3d'];
  const board: Square[] = [
    { row: 9, col: 4, piece: { type: 'General', color: 'black' } },
    { row: 0, col: 4, piece: { type: 'General', color: 'red' } },
  ];

  for (const side of ['red', 'black'] as const) {
    // Test hitting the top center of Black General
    const blackCoords = getVisualCoords(9, 4, side);
    const blackTop = projectBoardPoint(blackCoords.x, blackCoords.y, cam3d.pieceHeight, cam3d);
    assert.deepEqual(
      pickBoardSquare(blackTop.x, blackTop.y, side, cam3d, board),
      [9, 4]
    );

    // Test hitting the top center of Red General (when selected & lifted)
    const redCoords = getVisualCoords(0, 4, side);
    const redLiftedTop = projectBoardPoint(
      redCoords.x,
      redCoords.y,
      cam3d.pieceHeight * 1.65,
      cam3d
    );
    assert.deepEqual(
      pickBoardSquare(redLiftedTop.x, redLiftedTop.y, side, cam3d, board, {
        selectedSquare: [0, 4],
      }),
      [0, 4]
    );

    // Test hitting an empty intersection on z=0
    const emptyCoords = getVisualCoords(4, 2, side);
    const emptyProj = projectBoardPoint(emptyCoords.x, emptyCoords.y, 0, cam3d);
    assert.deepEqual(
      pickBoardSquare(emptyProj.x, emptyProj.y, side, cam3d, board),
      [4, 2]
    );
  }
});
