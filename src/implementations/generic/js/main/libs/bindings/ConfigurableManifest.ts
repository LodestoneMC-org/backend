// This file was generated by [ts-rs](https://github.com/Aleph-Alpha/ts-rs). Do not edit this file manually.
import type { SectionManifest } from "./SectionManifest.ts";

export interface ConfigurableManifest {
    auto_start: boolean;
    restart_on_crash: boolean;
    start_on_connection: boolean;
    setting_sections: Record<string, SectionManifest>;
}
