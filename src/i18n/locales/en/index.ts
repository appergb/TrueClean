// en locale — aggregates all namespaces. B1 maintains this file;
// B2/B3/B4 only edit their own namespace files (scan/cleanup/agent).
import { agent } from "./agent";
import { lens } from "./lens";
import { permissions } from "./permissions";
import { scan } from "./scan";
import { settings } from "./settings";
import { shell } from "./shell";

export const en = { shell, scan, agent, lens, permissions, settings };
export default en;
