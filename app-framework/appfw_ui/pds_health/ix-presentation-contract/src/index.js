export const pdsIxPresentationSchema = "pds.ix.presentation@1";

export {
  assertPdsIxRecipeRegistration,
  getPdsIxRecipe,
  pdsIxRecipeCapabilities,
  pdsIxRecipeIds,
  pdsIxRecipeRegistrationSchema,
  pdsIxRecipeRegistry,
  pdsIxRecipeRegistrySchema,
  projectPdsIxRecipeRegistration,
  validatePdsIxRecipeRegistration
} from "./recipes.js";

const statusValues = new Set(["partial", "ready", "stale", "failed"]);

function isRecord(value) {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function checkClosedRecord(value, path, required, optional, errors) {
  if (!isRecord(value)) {
    errors.push(`${path} must be an object`);
    return false;
  }
  const allowed = new Set([...required, ...optional]);
  for (const key of Object.keys(value)) {
    if (!allowed.has(key)) errors.push(`${path}.${key} is not allowed`);
  }
  for (const key of required) {
    if (!(key in value)) errors.push(`${path}.${key} is required`);
  }
  return true;
}

function hasUnicodeCodePointLengthWithin(value, maximum) {
  let length = 0;
  for (const _codePoint of value) {
    length += 1;
    if (length > maximum) return false;
  }
  return length > 0;
}

function checkString(value, path, errors) {
  if (
    typeof value !== "string"
    || !hasUnicodeCodePointLengthWithin(value, 4096)
  ) {
    errors.push(`${path} must be a string between 1 and 4096 characters`);
  }
}

function checkIdentifier(value, path, errors) {
  if (
    typeof value !== "string"
    || !hasUnicodeCodePointLengthWithin(value, 128)
  ) {
    errors.push(`${path} must be a string between 1 and 128 characters`);
  }
}

function checkOptionalString(value, path, errors) {
  if (value !== undefined) checkString(value, path, errors);
}

function checkStringArray(value, path, limit, errors) {
  if (!Array.isArray(value)) {
    errors.push(`${path} must be an array`);
    return;
  }
  if (value.length > limit) errors.push(`${path} must contain at most ${limit} items`);
  value.forEach((item, index) => checkString(item, `${path}[${index}]`, errors));
}

function checkEvidence(value, path, errors) {
  if (!checkClosedRecord(value, path, ["summary", "items"], [], errors)) return;
  checkString(value.summary, `${path}.summary`, errors);
  if (!Array.isArray(value.items)) {
    errors.push(`${path}.items must be an array`);
    return;
  }
  if (value.items.length > 64) errors.push(`${path}.items must contain at most 64 items`);
  value.items.forEach((item, index) => {
    const itemPath = `${path}.items[${index}]`;
    if (!checkClosedRecord(item, itemPath, ["label", "value"], ["id", "sourceRef"], errors)) return;
    if (item.id !== undefined) checkIdentifier(item.id, `${itemPath}.id`, errors);
    if (item.sourceRef !== undefined) checkIdentifier(item.sourceRef, `${itemPath}.sourceRef`, errors);
    checkString(item.label, `${itemPath}.label`, errors);
    checkString(item.value, `${itemPath}.value`, errors);
  });
}

function checkContext(value, path, errors) {
  if (!checkClosedRecord(
    value,
    path,
    ["eyebrow", "title", "announcement", "gaps", "evidence"],
    ["detail", "metaLabel", "actionLabel"],
    errors
  )) return;
  checkString(value.eyebrow, `${path}.eyebrow`, errors);
  checkString(value.title, `${path}.title`, errors);
  checkOptionalString(value.detail, `${path}.detail`, errors);
  checkString(value.announcement, `${path}.announcement`, errors);
  checkStringArray(value.gaps, `${path}.gaps`, 32, errors);
  checkEvidence(value.evidence, `${path}.evidence`, errors);
  checkOptionalString(value.metaLabel, `${path}.metaLabel`, errors);
  checkOptionalString(value.actionLabel, `${path}.actionLabel`, errors);
}

function checkWorkStatus(value, path, errors) {
  if (!checkClosedRecord(value, path, ["label", "active"], ["detail", "actionLabel"], errors)) return;
  checkString(value.label, `${path}.label`, errors);
  checkOptionalString(value.detail, `${path}.detail`, errors);
  if (typeof value.active !== "boolean") errors.push(`${path}.active must be a boolean`);
  checkOptionalString(value.actionLabel, `${path}.actionLabel`, errors);
}

function checkRegion(value, path, errors) {
  if (!checkClosedRecord(
    value,
    path,
    ["id", "status", "title", "body"],
    ["label", "whyItMatters", "evidence", "actionLabel", "editable", "changed"],
    errors
  )) return;
  checkIdentifier(value.id, `${path}.id`, errors);
  if (!statusValues.has(value.status)) errors.push(`${path}.status is not supported`);
  checkOptionalString(value.label, `${path}.label`, errors);
  checkString(value.title, `${path}.title`, errors);
  checkString(value.body, `${path}.body`, errors);
  checkOptionalString(value.whyItMatters, `${path}.whyItMatters`, errors);
  if (value.evidence !== undefined) checkEvidence(value.evidence, `${path}.evidence`, errors);
  checkOptionalString(value.actionLabel, `${path}.actionLabel`, errors);
  if (value.editable !== undefined && typeof value.editable !== "boolean") {
    errors.push(`${path}.editable must be a boolean`);
  }
  if (value.changed !== undefined && typeof value.changed !== "boolean") {
    errors.push(`${path}.changed must be a boolean`);
  }
}

function checkResponse(value, path, errors) {
  if (!checkClosedRecord(
    value,
    path,
    ["eyebrow", "title", "announcement", "active", "regions", "emptyState"],
    ["metaLabel", "footerText"],
    errors
  )) return;
  checkString(value.eyebrow, `${path}.eyebrow`, errors);
  checkString(value.title, `${path}.title`, errors);
  checkOptionalString(value.metaLabel, `${path}.metaLabel`, errors);
  checkString(value.announcement, `${path}.announcement`, errors);
  if (typeof value.active !== "boolean") errors.push(`${path}.active must be a boolean`);
  if (!Array.isArray(value.regions)) {
    errors.push(`${path}.regions must be an array`);
  } else {
    if (value.regions.length > 64) errors.push(`${path}.regions must contain at most 64 items`);
    value.regions.forEach((region, index) => checkRegion(region, `${path}.regions[${index}]`, errors));
    const regionIds = new Set();
    value.regions.forEach((region, index) => {
      if (typeof region?.id !== "string") return;
      if (regionIds.has(region.id)) {
        errors.push(`${path}.regions[${index}].id must be unique within the response`);
      }
      regionIds.add(region.id);
    });
  }
  checkString(value.emptyState, `${path}.emptyState`, errors);
  checkOptionalString(value.footerText, `${path}.footerText`, errors);
}

export function validatePdsIxPresentation(value) {
  const errors = [];
  if (checkClosedRecord(value, "$", ["identity", "announcement", "response"], ["context", "workStatus"], errors)) {
    if (checkClosedRecord(value.identity, "$.identity", ["schemaVersion", "presentationId", "revision"], [], errors)) {
      if (value.identity.schemaVersion !== pdsIxPresentationSchema) {
        errors.push(`$.identity.schemaVersion must equal ${pdsIxPresentationSchema}`);
      }
      checkIdentifier(value.identity.presentationId, "$.identity.presentationId", errors);
      if (
        !Number.isSafeInteger(value.identity.revision)
        || value.identity.revision < 0
      ) {
        errors.push("$.identity.revision must be a non-negative safe integer");
      }
    }
    checkString(value.announcement, "$.announcement", errors);
    if (value.context !== undefined) checkContext(value.context, "$.context", errors);
    if (value.workStatus !== undefined) checkWorkStatus(value.workStatus, "$.workStatus", errors);
    checkResponse(value.response, "$.response", errors);
  }
  return errors.length === 0
    ? { ok: true, value, errors: [] }
    : { ok: false, value: null, errors };
}

export function assertPdsIxPresentation(value) {
  const result = validatePdsIxPresentation(value);
  if (!result.ok) {
    throw new TypeError(`Invalid ${pdsIxPresentationSchema}: ${result.errors.join("; ")}`);
  }
  return result.value;
}
