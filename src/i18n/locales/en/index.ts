// en locale — aggregates all namespaces. B1 maintains this file;
// B2/B3/B4 only edit their own namespace files (scan/cleanup/agent).
import { agent } from "./agent";
import { cleanup } from "./cleanup";
import { lens } from "./lens";
import { permissions } from "./permissions";
import { scan } from "./scan";
import { shell } from "./shell";

export const en = { shell, scan, cleanup, agent, lens, permissions };
export default en;
