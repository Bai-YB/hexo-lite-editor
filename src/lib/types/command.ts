export interface CommandResult {
  success: boolean;
  command: string;
  stdout: string;
  stderr: string;
  code?: number;
}
