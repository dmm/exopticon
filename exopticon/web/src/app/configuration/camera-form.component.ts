import { Component, OnInit, input, output } from "@angular/core";
import { FormsModule } from "@angular/forms";
import { RouterLink } from "@angular/router";
import { Camera, ResourceMetadata } from "../camera";
import {
  ConfigurationFieldError,
  ConfigurationForm,
  FieldErrors,
  MetadataPatch,
  numberError,
} from "./configuration-form";

const defaultSettings = {
  enabled: false,
  ip: "",
  onvifPort: 80 as number | null,
  mac: "",
  ptzType: "none",
  onvifProfileToken: "",
  ptzXStepSize: 1 as number | null,
  ptzYStepSize: 1 as number | null,
};

function changes<T extends object>(before: T, after: T): Partial<T> {
  const patch: Partial<T> = {};
  for (const key of Object.keys(after) as (keyof T)[]) {
    if (before[key] !== after[key]) {
      patch[key] = after[key];
    }
  }
  return patch;
}

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
  settings = { ...defaultSettings };
  private originalSettings = { ...defaultSettings };
  storageGroupName = "";
  rtspUrl = "";
  username = "";
  password = "";
  validation: FieldErrors = {};

  ngOnInit(): void {
    const spec = this.camera()?.spec;
    if (spec) {
      this.storageGroupName = spec.storageGroupName;
      this.settings = {
        enabled: spec.enabled,
        ip: spec.ip,
        onvifPort: spec.onvifPort,
        mac: spec.mac,
        ptzType: spec.ptzType,
        onvifProfileToken: spec.onvifProfileToken ?? "",
        ptzXStepSize: spec.ptzXStepSize,
        ptzYStepSize: spec.ptzYStepSize,
      };
      this.originalSettings = { ...this.settings };
    }
  }

  error(field: string): string {
    return this.validation[field] || this.errors()[field];
  }

  save(metadata: MetadataPatch | null): void {
    const creating = this.camera() === null;
    const patch = creating
      ? this.settings
      : changes(this.originalSettings, this.settings);
    this.validation = {
      storageGroupName:
        creating && !this.storageGroupName ? "Enter a value." : "",
      rtspUrl: creating && !this.rtspUrl ? "Enter a value." : "",
      onvifPort:
        patch.onvifPort === undefined
          ? ""
          : numberError(patch.onvifPort, 1, 65535),
      ptzXStepSize:
        patch.ptzXStepSize === undefined
          ? ""
          : numberError(patch.ptzXStepSize, -32768, 32767),
      ptzYStepSize:
        patch.ptzYStepSize === undefined
          ? ""
          : numberError(patch.ptzYStepSize, -32768, 32767),
    };
    if (!metadata || Object.values(this.validation).some(Boolean)) {
      return;
    }

    this.saved.emit({
      metadata,
      spec: {
        ...patch,
        ...(creating ? { storageGroupName: this.storageGroupName } : {}),
        ...(this.rtspUrl ? { rtspUrl: this.rtspUrl } : {}),
        ...(this.username ? { username: this.username } : {}),
        ...(this.password ? { password: this.password } : {}),
      },
    });
  }
}
