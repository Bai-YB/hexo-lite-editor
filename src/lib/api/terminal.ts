import { command } from "./tauri";
import type { CommandResult } from "$lib/types/command";

export function runTerminalCommand(projectPath: string, terminalCommand: string): Promise<CommandResult> {
  return command<CommandResult>("run_terminal_command", { projectPath, command: terminalCommand });
}
