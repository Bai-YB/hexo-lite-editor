export interface SettingsController {
  save(): Promise<void>;
  discard(): void | Promise<void>;
  hasDirty(): boolean;
}
