import * as path from 'path';
import * as fs from 'fs';
import Mocha from 'mocha';

export function run(): Promise<void> {
  const mocha = new Mocha({
    ui: 'tdd',
    color: true,
  });

  const suiteDir = __dirname;

  return new Promise((c, e) => {
    try {
      const files = fs.readdirSync(suiteDir);
      const testFiles = files.filter((f) => f.endsWith('.test.js'));

      for (const f of testFiles) {
        mocha.addFile(path.resolve(suiteDir, f));
      }

      mocha.run((failures) => {
        if (failures > 0) {
          e(new Error(`${failures} tests failed.`));
        } else {
          c();
        }
      });
    } catch (err) {
      e(err);
    }
  });
}
