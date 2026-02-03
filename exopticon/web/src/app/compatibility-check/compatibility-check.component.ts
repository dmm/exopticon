import { AsyncPipe, KeyValuePipe, NgClass } from "@angular/common";
import { Component } from "@angular/core";
import { from, Observable } from "rxjs";

export const H265_VARIANTS = {
  // Main Profile
  main_30: "hev1.1.6.L90.B0", // Level 3.0
  main_31: "hev1.1.6.L93.B0", // Level 3.1
  main_40: "hev1.1.6.L120.B0", // Level 4.0
  main_41: "hev1.1.6.L123.B0", // Level 4.1
  main_50: "hev1.1.6.L150.B0", // Level 5.0
  main_51: "hev1.1.6.L153.B0", // Level 5.1
  main_52: "hev1.1.6.L156.B0", // Level 5.2

  // Main 10 Profile (10-bit color)
  main10_40: "hev1.2.4.L120.B0",
  main10_50: "hev1.2.4.L150.B0",
  main10_51: "hev1.2.4.L153.B0",
} as const;

const CODEC_STRINGS = new Map<string, string>([
  ["H.264 Baseline, Level 3.0", "avc1.42001E"],
  ["H.264 Baseline, Level 3.1", "avc1.42001F"],
  ["H.264 Baseline, Level 4.0", "avc1.420028"],

  ["H.264 Main Profile, Level 3.0", "avc1.4D001E"],
  ["H.264 Main Profile, Level 3.1", "avc1.4D001F"],
  ["H.264 Main Profile, Level 4.0", "avc1.4D0028"],
  ["H.264 Main Profile, Level 4.1", "avc1.4D0029"],

  ["H.264 High Profile, Level 3.0", "avc1.64001E"],
  ["H.264 High Profile, Level 3.0", "avc1.64001F"],
  ["H.264 High Profile, Level 4.0", "avc1.640028"],
  ["H.264 High Profile, Level 4.1", "avc1.640029"],
  ["H.264 High Profile, Level 5.0 (4K)", "avc1.640032"],
  ["H.264 High Profile, Level 5.1 (4K)", "avc1.640033"],
  ["H.264 High Profile, Level 5.2 (4K high fps)", "avc1.640034"],

  ["H.265 Main Profile, Level 3.0", "hev1.1.6.L90.B0"],
  ["H.265 Main Profile, Level 3.1", "hev1.1.6.L93.B0"],
  ["H.265 Main Profile, Level 4.0", "hev1.1.6.L120.B0"],
  ["H.265 Main Profile, Level 4.1", "hev1.1.6.L123.B0"],
  ["H.265 Main Profile, Level 5.0", "hev1.1.6.L150.B0"],
  ["H.265 Main Profile, Level 5.1", "hev1.1.6.L153.B0"],
  ["H.265 Main Profile, Level 5.2", "hev1.1.6.L156.B0"],
]);

async function checkCodecSupport(codec: string): Promise<boolean> {
  if (typeof VideoDecoder === "undefined") {
    return false;
  }

  try {
    const support = await VideoDecoder.isConfigSupported({
      codec,
      codedWidth: 1920,
      codedHeight: 1080,
    });
    return support.supported === true;
  } catch {
    return false;
  }
}

async function checkCodecs(): Promise<Map<string, boolean>> {
  let supportMap = new Map<string, boolean>();
  for (let [codecName, codecId] of CODEC_STRINGS) {
    let supported = await checkCodecSupport(codecId);
    supportMap.set(codecName, supported);
  }

  return supportMap;
}

@Component({
  selector: "app-compatibility-check",
  imports: [KeyValuePipe, AsyncPipe, NgClass],
  templateUrl: "./compatibility-check.component.html",
  styleUrl: "./compatibility-check.component.css",
})
export class CompatibilityCheckComponent {
  public webTransportSupported: boolean;
  public webCodecSupported: boolean;
  public codecSupport: Map<string, string> = new Map();
  public supportedCodecs$: Observable<Map<string, boolean>>;
  ngOnInit() {
    this.webTransportSupported = typeof WebTransport !== "undefined";
    this.webCodecSupported =
      typeof VideoDecoder !== "undefined" &&
      typeof AudioDecoder !== "undefined";
    this.supportedCodecs$ = from(checkCodecs());
  }
}
