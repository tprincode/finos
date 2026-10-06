import { CALCULATOR_COLUMN_INTENT } from "./calculatorColumns";
import { CONTRACT_FIELD_INTENT } from "./contractFields";

export type FieldIntentRow = {
  name: string;
  intent: string;
  formula: string;
  status: string;
  pageId: string;
  componentId: string;
  owner: string;
};

export const ALL_FIELD_INTENT: readonly FieldIntentRow[] = [
  ...CALCULATOR_COLUMN_INTENT.map((row) => ({
    ...row,
    pageId: "calculator",
    componentId: "calculator-columns",
    owner: "Calculator columns",
  })),
  ...CONTRACT_FIELD_INTENT,
];

export function fieldNamesFor(moduleId: string, partId: string): string {
  const names = ALL_FIELD_INTENT.filter(
    (row) => row.componentId === partId || (partId === "" && row.pageId === moduleId),
  ).map((row) => row.name);
  return names.join(", ");
}
