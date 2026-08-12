import { readFile } from "node:fs/promises";

const mappings = JSON.parse(await readFile("compatibility/wpt-mappings.json", "utf8"));
const paths = new Set();
for (const mapping of mappings.mappings) {
  if (mapping.disposition === "excluded") continue;
  paths.add(mapping.path);
}

for (const path of [...paths].sort()) {
  console.log(path);
}
