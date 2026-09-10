export type FieldSchema = Record<string, unknown>;
export function settingFields(schema: Record<string, unknown> | null): Array<[string, FieldSchema]> {
  const properties = schema?.properties;
  return properties && typeof properties === "object" && !Array.isArray(properties)
    ? Object.entries(properties).filter((entry): entry is [string, FieldSchema] => !!entry[1] && typeof entry[1] === "object" && !Array.isArray(entry[1]))
    : [];
}

export function initializeSettings(schema: Record<string, unknown> | null, saved: Record<string, unknown>) {
  const result = { ...saved };
  for (const [key, field] of settingFields(schema)) {
    if (!(key in result) && "default" in field) result[key] = structuredClone(field.default);
  }
  return result;
}

export function validateSettings(schema: Record<string, unknown> | null, values: Record<string, unknown>): Record<string, string> {
  const errors: Record<string, string> = {};
  const required = Array.isArray(schema?.required) ? schema.required : [];
  for (const [key, field] of settingFields(schema)) {
    const value = values[key];
    if (value === undefined || value === "") {
      if (required.includes(key)) errors[key] = "请填写此项。";
      continue;
    }
    if (Array.isArray(field.enum) && !field.enum.includes(value)) errors[key] = "请选择有效选项。";
    else if ((field.type === "number" || field.type === "integer") && (typeof value !== "number" || !Number.isFinite(value) || (field.type === "integer" && !Number.isInteger(value)))) errors[key] = "请输入有效数字。";
    else if (field.type === "boolean" && typeof value !== "boolean") errors[key] = "请选择开启或关闭。";
    else if (field.type === "string" && typeof value !== "string") errors[key] = "请输入文本。";
    else if (typeof value === "number" && ((typeof field.minimum === "number" && value < field.minimum) || (typeof field.maximum === "number" && value > field.maximum))) errors[key] = "数值超出允许范围。";
    else if (typeof value === "string" && ((typeof field.minLength === "number" && value.length < field.minLength) || (typeof field.maxLength === "number" && value.length > field.maxLength))) errors[key] = "文本长度不符合要求。";
    else if (field.format === "uri" && typeof value === "string") {
      try { new URL(value); } catch { errors[key] = "请输入完整有效的地址。"; }
    }
  }
  return errors;
}
