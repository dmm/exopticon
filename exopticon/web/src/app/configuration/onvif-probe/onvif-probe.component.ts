import { HttpClient, HttpErrorResponse } from "@angular/common/http";
import { Component, DestroyRef, inject, output, signal } from "@angular/core";
import { takeUntilDestroyed } from "@angular/core/rxjs-interop";
import { FormsModule } from "@angular/forms";

interface ProbeProfile {
  name: string;
  token: string;
  rtspUrl: string | null;
  snapshotUrl: string | null;
  ptzSupported: boolean;
  video: {
    codec: string;
    width: number;
    height: number;
    frameRate: number | null;
  } | null;
}

interface ProbeResult {
  deviceServiceUrl: string;
  device: {
    manufacturer: string;
    model: string;
    firmwareVersion: string;
    serialNumber: string;
    hardwareId: string;
  } | null;
  profiles: ProbeProfile[];
  profileError: string | null;
}

export interface ProbedCameraFields {
  onvifProfileToken: string;
  onvifDeviceUrl: string;
  username: string;
  password: string;
}

@Component({
  selector: "app-onvif-probe",
  imports: [FormsModule],
  templateUrl: "./onvif-probe.component.html",
  styleUrl: "./onvif-probe.component.css",
})
export class OnvifProbeComponent {
  private readonly http = inject(HttpClient);
  private readonly destroyRef = inject(DestroyRef);
  private credentials = { username: "", password: "" };

  readonly profileSelected = output<ProbedCameraFields>();
  readonly probing = signal(false);
  readonly error = signal("");
  readonly result = signal<ProbeResult | null>(null);

  host = "";
  port: number | null = 80;
  username = "";
  password = "";

  probe(): void {
    if (this.probing()) {
      return;
    }
    this.error.set("");
    this.result.set(null);
    const host = this.host.trim();
    const port = this.port ?? 80;
    if (!host) {
      this.error.set("Enter the camera's hostname or IPv4 address.");
      return;
    }
    if (!Number.isInteger(port) || port < 1 || port > 65535) {
      this.error.set("Enter a port from 1 to 65535.");
      return;
    }

    const { username, password } = this;
    this.probing.set(true);
    this.http
      .post<ProbeResult>("/v1/cameras/onvif/probe", {
        host,
        port,
        username,
        password,
      })
      .pipe(takeUntilDestroyed(this.destroyRef))
      .subscribe({
        next: (result) => {
          this.credentials = { username, password };
          this.result.set(result);
          this.probing.set(false);
        },
        error: (error: unknown) => {
          this.error.set(this.errorMessage(error));
          this.probing.set(false);
        },
      });
  }

  visibleRtspUrl(url: string): string {
    const visible = new URL(url);
    visible.username = "";
    visible.password = "";
    return visible.href;
  }

  selectProfile(profile: ProbeProfile): void {
    const result = this.result();
    if (!result) {
      return;
    }
    this.profileSelected.emit({
      onvifDeviceUrl: result.deviceServiceUrl,
      onvifProfileToken: profile.token,
      ...this.credentials,
    });
  }

  private errorMessage(error: unknown): string {
    if (error instanceof HttpErrorResponse && error.status === 422) {
      if (typeof error.error === "string") {
        return error.error;
      }
      if (typeof error.error?.message === "string") {
        return error.error.message;
      }
    }
    return "The ONVIF probe failed. Try again.";
  }
}
