import { Component, OnInit, input, output, viewChild } from "@angular/core";
import { FormsModule } from "@angular/forms";
import { RouterLink } from "@angular/router";
import { Camera, PtzSettings, ResourceMetadata, StreamSource } from "../camera";
import {
  ConfigurationFieldError,
  ConfigurationForm,
  FieldErrors,
  MetadataPatch,
  numberError,
} from "./configuration-form";
import { ProbedCameraFields } from "./onvif-probe/onvif-probe.component";

@Component({
  selector: "app-camera-configuration-form",
  imports: [
    FormsModule,
    RouterLink,
    ConfigurationForm,
    ConfigurationFieldError,
  ],
  templateUrl: "./camera-form.component.html",
})
export class CameraFormComponent implements OnInit {
  readonly camera = input.required<Camera | null>();
  readonly storageGroups = input.required<ResourceMetadata[]>();
  readonly listLink = input.required<string[]>();
  readonly errors = input.required<FieldErrors>();
  readonly saving = input.required<boolean>();
  readonly saved = output<object>();
  private readonly form = viewChild.required(ConfigurationForm);
  streamType: StreamSource["type"] = "staticRtsp";
  storageGroupName = "";
  enabled = false;
  rtspUrl = "";
  onvifDeviceUrl = "";
  onvifProfileToken = "";
  ptzEnabled = false;
  ptzDeviceUrl = "";
  ptzProfileToken = "";
  ptzXStepSize: number | null = 1;
  ptzYStepSize: number | null = 1;
  username = "";
  password = "";
  validation: FieldErrors = {};

  get currentRtspUrl(): string {
    const source = this.camera()?.spec.streamSource;
    return source?.type === "staticRtsp" ? source.url : "";
  }

  ngOnInit(): void {
    const spec = this.camera()?.spec;
    if (!spec) {
      return;
    }
    this.storageGroupName = spec.storageGroupName;
    this.enabled = spec.enabled;
    this.streamType = spec.streamSource.type;
    if (spec.streamSource.type === "onvif") {
      this.onvifDeviceUrl = spec.streamSource.deviceUrl;
      this.onvifProfileToken = spec.streamSource.profileToken;
    }
    if (spec.ptz) {
      this.ptzEnabled = true;
      this.ptzDeviceUrl = spec.ptz.deviceUrl ?? "";
      this.ptzProfileToken = spec.ptz.profileToken ?? "";
      this.ptzXStepSize = spec.ptz.xStepSize;
      this.ptzYStepSize = spec.ptz.yStepSize;
    }
  }

  error(field: string): string {
    return this.validation[field] || this.errors()[field];
  }

  applyOnvifProfile(fields: ProbedCameraFields): void {
    this.streamType = "onvif";
    this.onvifDeviceUrl = fields.onvifDeviceUrl;
    this.onvifProfileToken = fields.onvifProfileToken;
    this.username = fields.username;
    this.password = fields.password;
    this.validation = {};
    this.form().focusName();
  }

  save(metadata: MetadataPatch | null): void {
    const original = this.camera()?.spec;
    const rtspUrl = this.rtspUrl.trim();
    const onvif = this.streamType === "onvif";
    const stream: StreamSource = onvif
      ? {
          type: "onvif",
          deviceUrl: this.onvifDeviceUrl.trim(),
          profileToken: this.onvifProfileToken,
        }
      : { type: "staticRtsp", url: rtspUrl || this.currentRtspUrl };
    const sourceChanged =
      stream.type !== original?.streamSource.type ||
      (stream.type === "onvif" && original.streamSource.type === "onvif"
        ? stream.deviceUrl !== original.streamSource.deviceUrl ||
          stream.profileToken !== original.streamSource.profileToken
        : rtspUrl !== "" && rtspUrl !== this.currentRtspUrl);

    const ptz: PtzSettings | null = this.ptzEnabled
      ? {
          deviceUrl: this.ptzDeviceUrl.trim() || null,
          profileToken: this.ptzProfileToken.trim()
            ? this.ptzProfileToken
            : null,
          xStepSize: this.ptzXStepSize ?? 1,
          yStepSize: this.ptzYStepSize ?? 1,
        }
      : null;
    const previousPtz = original?.ptz ?? null;
    const ptzChanged =
      ptz?.deviceUrl !== previousPtz?.deviceUrl ||
      ptz?.profileToken !== previousPtz?.profileToken ||
      this.ptzXStepSize !== (previousPtz?.xStepSize ?? 1) ||
      this.ptzYStepSize !== (previousPtz?.yStepSize ?? 1);
    const requirePtzTarget = ptz && !onvif && (sourceChanged || ptzChanged);

    this.validation = {
      storageGroupName:
        !original && !this.storageGroupName ? "Enter a value." : "",
      rtspUrl:
        !original && stream.type === "staticRtsp" && !stream.url
          ? "Enter an RTSP URL."
          : "",
      onvifDeviceUrl:
        stream.type === "onvif" && !stream.deviceUrl
          ? "Enter a device service URL."
          : "",
      onvifProfileToken:
        stream.type === "onvif" && !stream.profileToken.trim()
          ? "Enter a profile token."
          : "",
      ptzDeviceUrl:
        requirePtzTarget && !ptz.deviceUrl
          ? "Enter a PTZ device service URL."
          : "",
      ptzProfileToken:
        requirePtzTarget && !ptz.profileToken
          ? "Enter a PTZ profile token."
          : "",
      ptzXStepSize: ptz ? numberError(this.ptzXStepSize, -32768, 32767) : "",
      ptzYStepSize: ptz ? numberError(this.ptzYStepSize, -32768, 32767) : "",
    };
    if (!metadata || Object.values(this.validation).some(Boolean)) {
      return;
    }

    this.saved.emit({
      metadata,
      spec: {
        ...(!original ? { storageGroupName: this.storageGroupName } : {}),
        ...(original?.enabled !== this.enabled
          ? { enabled: this.enabled }
          : {}),
        ...(sourceChanged ? { streamSource: stream } : {}),
        ...(ptzChanged ? { ptz } : {}),
        ...(this.username ? { username: this.username } : {}),
        ...(this.password ? { password: this.password } : {}),
      },
    });
  }
}
