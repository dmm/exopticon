import { Component, OnInit } from "@angular/core";
import { CommonModule } from "@angular/common";

interface CodecInfo {
  mimeType: string;
  clockRate: number;
  channels?: number;
  sdpFmtpLine?: string;
}

@Component({
  selector: "app-webrtc-codecs",
  standalone: true,
  imports: [CommonModule],
  templateUrl: "./webrtc-codecs.component.html",
  styleUrls: ["./webrtc-codecs.component.css"],
})
export class WebrtcCodecsComponent implements OnInit {
  audioCodecs: CodecInfo[] = [];
  videoCodecs: CodecInfo[] = [];
  supported = false;

  ngOnInit(): void {
    if (
      typeof RTCRtpReceiver !== "undefined" &&
      RTCRtpReceiver.getCapabilities
    ) {
      this.supported = true;
      this.loadCodecs();
    }
  }

  private loadCodecs(): void {
    const audioCapabilities = RTCRtpReceiver.getCapabilities("audio");
    const videoCapabilities = RTCRtpReceiver.getCapabilities("video");

    if (audioCapabilities) {
      this.audioCodecs = audioCapabilities.codecs.map((codec) => ({
        mimeType: codec.mimeType,
        clockRate: codec.clockRate,
        channels: codec.channels ?? undefined,
        sdpFmtpLine: codec.sdpFmtpLine ?? undefined,
      }));
    }

    if (videoCapabilities) {
      this.videoCodecs = videoCapabilities.codecs.map((codec) => ({
        mimeType: codec.mimeType,
        clockRate: codec.clockRate,
        sdpFmtpLine: codec.sdpFmtpLine ?? undefined,
      }));
    }
  }
}
