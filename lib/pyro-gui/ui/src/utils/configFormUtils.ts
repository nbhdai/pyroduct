/**
 * Shared utility functions for rendering PyroSchema-based forms.
 * Used by CallPlaybookForm, CapabilityConfigForm, and CapabilitySpecView.
 */

export const renderType = (type: any): string => {
  if (!type) return "Unknown";
  if (typeof type === "string") return type;
  if (type && typeof type === "object") {
    if (type.PrimitiveScalar) return type.PrimitiveScalar;
    if (type.PrimitiveList) return `[${type.PrimitiveList}]`;
    if (type.PrimitiveFixedList) return `[${type.PrimitiveFixedList[0]}; ${type.PrimitiveFixedList[1]}]`;
    if (type.List) return `[${renderType(type.List[0])}]`;
    if (type.Map) return `Map<${renderType(type.Map.key)}, ${renderType(type.Map.value)}>`;
    if (type.Group) {
      return `{ ${type.Group.map((f: any) => `${f.name}: ${renderType(f.data_type)}`).join(", ")} }`;
    }
    return JSON.stringify(type);
  }
  return "Unknown";
};

export const isGroupType = (type: any): boolean => {
  return !!(type && typeof type === "object" && type.Group);
};

export const isComplexType = (type: any): boolean => {
  if (!type) return false;
  if (typeof type === "string") return false;
  if (typeof type === "object") {
    if (type.PrimitiveScalar) return false;
    if (type.Group) return false; // Handled separately as individual fields
    return true;
  }
  return false;
};

export const getDefaultValueForType = (type: any): any => {
  if (!type) return null;
  if (typeof type === "string") {
    switch (type) {
      case "Null": return null;
      case "Str": return "";
      case "Timestamp": return new Date().toISOString();
      default: return null;
    }
  }
  if (typeof type === "object") {
    if (type.PrimitiveScalar) {
      const scalar = type.PrimitiveScalar;
      if (scalar === "Bool") return false;
      return 0;
    }
    if (type.PrimitiveList) {
      return [];
    }
    if (type.PrimitiveFixedList) {
      const [elemType, size] = type.PrimitiveFixedList;
      const val = elemType === "Bool" ? false : 0;
      return Array(size).fill(val);
    }
    if (type.List) {
      return [];
    }
    if (type.Map) {
      return {};
    }
    if (type.Group) {
      const obj: Record<string, any> = {};
      const fields = type.Group || [];
      fields.forEach((field: any) => {
        if (field.nullable) {
          obj[field.name] = null;
        } else {
          obj[field.name] = getDefaultValueForType(field.data_type);
        }
      });
      return obj;
    }
  }
  return null;
};

export const getInitialValueForType = (type: any): any => {
  if (isGroupType(type)) {
    const obj: Record<string, any> = {};
    const fields = type.Group || [];
    fields.forEach((field: any) => {
      if (field.nullable) {
        obj[field.name] = null;
      } else {
        obj[field.name] = getInitialValueForType(field.data_type);
      }
    });
    return obj;
  } else if (isComplexType(type)) {
    return JSON.stringify(getDefaultValueForType(type), null, 2);
  } else {
    return getDefaultValueForType(type);
  }
};

export const getValueAtPath = (obj: any, path: string[]): any => {
  let current = obj;
  for (const key of path) {
    if (current === undefined || current === null) return undefined;
    current = current[key];
  }
  return current;
};

export const setValueAtPath = (obj: any, path: string[], value: any): any => {
  const newObj = { ...obj };
  let current = newObj;
  for (let i = 0; i < path.length - 1; i++) {
    const key = path[i];
    current[key] = { ...current[key] };
    current = current[key];
  }
  current[path[path.length - 1]] = value;
  return newObj;
};

export const buildPayloadForType = (val: any, type: any, fieldName: string): any => {
  if (isGroupType(type)) {
    const obj: Record<string, any> = {};
    const subFields = type.Group || [];
    for (const subField of subFields) {
      const subVal = val ? val[subField.name] : undefined;
      obj[subField.name] = buildPayloadForType(subVal, subField.data_type, `${fieldName}.${subField.name}`);
    }
    return obj;
  }

  if (isComplexType(type)) {
    if (val === undefined || val === null || val === "") {
      return null;
    }
    try {
      return JSON.parse(val);
    } catch (err: any) {
      return null;
    }
  }

  // Primitive types
  if (typeof type === "string") {
    if (type === "Null") {
      return null;
    } else {
      return val !== undefined && val !== null ? String(val) : "";
    }
  }

  if (type && typeof type === "object" && type.PrimitiveScalar) {
    const scalar = type.PrimitiveScalar;
    if (scalar === "Bool") {
      return Boolean(val);
    } else {
      if (val === "" || val === undefined || val === null) {
        return null;
      }
      const num = Number(val);
      return isNaN(num) ? 0 : num;
    }
  }

  return val;
};
