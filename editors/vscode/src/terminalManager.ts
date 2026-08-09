import * as vscode from 'vscode';

export class TerminalManager implements vscode.Disposable {
  private currentTerminal: vscode.Terminal | undefined;
  private pollInterval: NodeJS.Timeout | undefined;
  private disposables: vscode.Disposable[] = [];

  constructor() {
    this.disposables.push(
      vscode.window.onDidCloseTerminal((terminal) => {
        if (terminal === this.currentTerminal) {
          this.stopPolling();
          this.currentTerminal = undefined;
        }
      })
    );
  }

  private stopPolling(): void {
    if (this.pollInterval) {
      clearInterval(this.pollInterval);
      this.pollInterval = undefined;
    }
  }

  private startExitMonitoring(terminal: vscode.Terminal): void {
    this.stopPolling();

    terminal.processId.then((pid) => {
      if (!pid || this.currentTerminal !== terminal) {
        return;
      }

      this.pollInterval = setInterval(() => {
        if (this.currentTerminal !== terminal) {
          this.stopPolling();
          return;
        }

        let isAlive = true;
        try {
          isAlive = process.kill(pid, 0);
        } catch (e: any) {
          isAlive = false;
        }

        if (!isAlive) {
          this.stopPolling();
          const config = vscode.workspace.getConfiguration('git-tardis');
          const autoClose = config.get<boolean>('autoClosePane', true);
          if (autoClose && this.currentTerminal === terminal) {
            this.currentTerminal.dispose();
            this.currentTerminal = undefined;
          }
        }
      }, 300);
    });
  }

  public launch(args: string[], cwd?: string): void {
    const config = vscode.workspace.getConfiguration('git-tardis');
    const binaryPath = config.get<string>('binaryPath', 'git-tardis');
    const locationSetting = config.get<string>('terminalLocation', 'editor');

    const location =
      locationSetting === 'panel'
        ? vscode.TerminalLocation.Panel
        : vscode.TerminalLocation.Editor;

    if (this.currentTerminal) {
      this.stopPolling();
      this.currentTerminal.dispose();
      this.currentTerminal = undefined;
    }

    const terminalOptions: vscode.TerminalOptions = {
      name: 'Git-tardis',
      shellPath: binaryPath,
      shellArgs: args,
      location: location,
      cwd: cwd,
    };

    const terminal = vscode.window.createTerminal(terminalOptions);
    this.currentTerminal = terminal;
    terminal.show();

    this.startExitMonitoring(terminal);
  }

  public toggle(getArgsAndCwd: () => { args: string[]; cwd?: string }): void {
    if (this.currentTerminal) {
      this.stopPolling();
      this.currentTerminal.dispose();
      this.currentTerminal = undefined;
    } else {
      const { args, cwd } = getArgsAndCwd();
      this.launch(args, cwd);
    }
  }

  public isRunning(): boolean {
    return this.currentTerminal !== undefined;
  }

  public dispose(): void {
    this.stopPolling();
    if (this.currentTerminal) {
      this.currentTerminal.dispose();
      this.currentTerminal = undefined;
    }
    for (const d of this.disposables) {
      d.dispose();
    }
    this.disposables = [];
  }
}
