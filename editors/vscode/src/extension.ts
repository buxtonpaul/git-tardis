import * as vscode from 'vscode';
import { TerminalManager } from './terminalManager';
import { registerCommands } from './commands';

let terminalManager: TerminalManager | undefined;

export function activate(context: vscode.ExtensionContext): void {
  terminalManager = new TerminalManager();
  context.subscriptions.push(terminalManager);

  registerCommands(context, terminalManager);
}

export function deactivate(): void {
  if (terminalManager) {
    terminalManager.dispose();
    terminalManager = undefined;
  }
}
