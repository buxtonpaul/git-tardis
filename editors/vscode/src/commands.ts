import * as vscode from 'vscode';
import { TerminalManager } from './terminalManager';

export interface ActiveContext {
  file?: string;
  line?: number;
  cwd?: string;
}

export function getActiveContext(): ActiveContext {
  const editor = vscode.window.activeTextEditor;
  if (!editor) {
    const workspaceFolders = vscode.workspace.workspaceFolders;
    return {
      cwd: workspaceFolders && workspaceFolders.length > 0 ? workspaceFolders[0].uri.fsPath : undefined,
    };
  }

  const document = editor.document;
  if (document.uri.scheme !== 'file' || document.isUntitled) {
    const workspaceFolder = vscode.workspace.getWorkspaceFolder(document.uri) || vscode.workspace.workspaceFolders?.[0];
    return {
      cwd: workspaceFolder ? workspaceFolder.uri.fsPath : undefined,
    };
  }

  const file = document.uri.fsPath;
  const line = editor.selection.active.line + 1; // Convert 0-based VS Code line index to 1-based CLI index
  const workspaceFolder = vscode.workspace.getWorkspaceFolder(document.uri) || vscode.workspace.workspaceFolders?.[0];
  const cwd = workspaceFolder ? workspaceFolder.uri.fsPath : undefined;

  return { file, line, cwd };
}

export function buildCmdArgs(ctx: ActiveContext, jumpMode?: string): string[] {
  const args: string[] = [];
  if (ctx.file) {
    args.push('--file', ctx.file);
  }
  if (ctx.line !== undefined) {
    args.push('--line', ctx.line.toString());
  }
  if (jumpMode) {
    args.push('--jump-mode', jumpMode);
  }
  return args;
}

export function registerCommands(
  context: vscode.ExtensionContext,
  terminalManager: TerminalManager
): void {
  const launchWithMode = (jumpMode?: string) => {
    const ctx = getActiveContext();
    const args = buildCmdArgs(ctx, jumpMode);
    terminalManager.launch(args, ctx.cwd);
  };

  context.subscriptions.push(
    vscode.commands.registerCommand('git-tardis.open', () => {
      launchWithMode();
    }),

    vscode.commands.registerCommand('git-tardis.toggle', () => {
      terminalManager.toggle(() => {
        const ctx = getActiveContext();
        return {
          args: buildCmdArgs(ctx),
          cwd: ctx.cwd,
        };
      });
    }),

    vscode.commands.registerCommand('git-tardis.inspectPrevFunction', () => {
      launchWithMode('function');
    }),

    vscode.commands.registerCommand('git-tardis.inspectPrevLine', () => {
      launchWithMode('line');
    }),

    vscode.commands.registerCommand('git-tardis.inspectPrevFile', () => {
      launchWithMode('file');
    }),

    vscode.commands.registerCommand('git-tardis.inspectPrevCommit', () => {
      launchWithMode('commit');
    })
  );
}
