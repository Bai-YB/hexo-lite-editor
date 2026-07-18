export interface SettingsController {
  save(): Promise<void>;
  discard(): void;
  hasDirty(): boolean;
}
