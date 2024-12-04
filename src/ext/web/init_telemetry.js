import * as telemetry from "ext:deno_telemetry/telemetry.ts";
import * as util from "ext:deno_telemetry/util.ts";

globalThis.Deno.telemetry = telemetry.telemetry;