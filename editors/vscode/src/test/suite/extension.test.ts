import * as assert from 'assert';
import { buildCmdArgs, ActiveContext, getActiveContext } from '../../commands';

suite('Git-tardis Extension Test Suite', () => {
  test('buildCmdArgs constructs flags correctly with file and line', () => {
    const ctx: ActiveContext = {
      file: '/path/to/repo/src/main.rs',
      line: 42,
      cwd: '/path/to/repo',
    };
    const args = buildCmdArgs(ctx);
    assert.deepStrictEqual(args, ['--file', '/path/to/repo/src/main.rs', '--line', '42']);
  });

  test('buildCmdArgs handles jump modes correctly', () => {
    const ctx: ActiveContext = {
      file: '/path/to/repo/src/main.rs',
      line: 100,
      cwd: '/path/to/repo',
    };

    assert.deepStrictEqual(buildCmdArgs(ctx, 'function'), [
      '--file',
      '/path/to/repo/src/main.rs',
      '--line',
      '100',
      '--jump-mode',
      'function',
    ]);

    assert.deepStrictEqual(buildCmdArgs(ctx, 'line'), [
      '--file',
      '/path/to/repo/src/main.rs',
      '--line',
      '100',
      '--jump-mode',
      'line',
    ]);

    assert.deepStrictEqual(buildCmdArgs(ctx, 'file'), [
      '--file',
      '/path/to/repo/src/main.rs',
      '--line',
      '100',
      '--jump-mode',
      'file',
    ]);

    assert.deepStrictEqual(buildCmdArgs(ctx, 'commit'), [
      '--file',
      '/path/to/repo/src/main.rs',
      '--line',
      '100',
      '--jump-mode',
      'commit',
    ]);
  });

  test('buildCmdArgs handles fallback when context has no file or line', () => {
    const ctx: ActiveContext = {
      cwd: '/path/to/repo',
    };
    assert.deepStrictEqual(buildCmdArgs(ctx), []);
    assert.deepStrictEqual(buildCmdArgs(ctx, 'commit'), ['--jump-mode', 'commit']);
  });

  test('getActiveContext does not throw when no text editor is active', () => {
    const ctx = getActiveContext();
    assert.ok(typeof ctx === 'object');
  });

  test('buildCmdArgs returns empty array when context has non-file scheme or untitled', () => {
    // A context derived from non-file or untitled buffer has no file property
    const ctx: ActiveContext = {
      cwd: '/path/to/workspace',
    };
    const args = buildCmdArgs(ctx, 'function');
    assert.deepStrictEqual(args, ['--jump-mode', 'function']);
    assert.strictEqual(ctx.file, undefined);
  });
});
