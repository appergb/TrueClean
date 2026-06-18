// en locale — aggregates all namespaces. B1 maintains this file;
// B2/B3/B4 only edit their own namespace files (scan/cleanup/agent).
import { shell } from "./shell";
import { scan } from "./scan";
import { cleanup } from "./cleanup";
import { agent } from "./agent";

export const en = { shell, scan, cleanup, agent };
export default en;
