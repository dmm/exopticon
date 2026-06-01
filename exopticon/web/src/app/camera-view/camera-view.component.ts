/*
 * Exopticon - A free video surveillance system.
 * Copyright (C) 2020 David Matthew Mattli <dmm@mattli.us>
 *
 * This file is part of Exopticon.
 *
 * Exopticon is free software: you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 *
 * Exopticon is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with Exopticon.  If not, see <http://www.gnu.org/licenses/>.
 */

import {
  ChangeDetectorRef,
  Component,
  ElementRef,
  EventEmitter,
  Input,
  OnInit,
  Output,
  SimpleChanges,
  ViewChild,
} from "@angular/core";
import { Observable, Subscription } from "rxjs";
import { Camera } from "../camera";
import { WebrtcService } from "../webrtc.service";
import { CameraPanelService } from "../camera-panel.service";
import { CameraOverlayComponent } from "../camera-overlay/camera-overlay.component";
import { CameraStatusOverlayComponent } from "../camera-status-overlay/camera-status-overlay.component";

export interface NewState {
  kind: "new";
}

export interface ConnectingState {
  kind: "connecting";
  lastTime: number;
}

export interface PlayingState {
  kind: "playing";
}

type CameraViewStatus = NewState | ConnectingState | PlayingState;

@Component({
  selector: "app-camera-view",
  templateUrl: "./camera-view.component.html",
  styleUrls: ["./camera-view.component.css"],
  imports: [CameraOverlayComponent, CameraStatusOverlayComponent],
})
export class CameraViewComponent implements OnInit {
  @Input() camera!: Camera;
  @Input() selected = false;
  @Input() enabled = false;

  @Output() isVisible = new EventEmitter<boolean>();

  @ViewChild("wrapperDiv") wrapperDiv!: ElementRef<HTMLDivElement>;

  @ViewChild("videoElement") videoElement!: ElementRef<HTMLVideoElement>;

  public status = "loading";
  public muted: boolean = true;

  private mediaStream?: MediaStream = undefined;
  private state: CameraViewStatus = { kind: "new" };
  private subscription: Subscription | null = null;

  constructor(
    private changeRef: ChangeDetectorRef,
    private webrtcService: WebrtcService,
    private cameraPanelService: CameraPanelService,
  ) {}

  ngOnInit() {
    if (this.enabled) {
      this.activate();
    }
  }

  ngAfterViewInit() {
    if (this.state.kind === "new") {
      this.onVideoStatusChange("loading...");
      this.activate();
      this.setMediaSource();
      this.state = {
        kind: "connecting",
        lastTime: this.getVideoElement().currentTime,
      };
    }
  }

  ngOnChanges(changes: SimpleChanges) {
    if (changes.hasOwnProperty("enabled")) {
      if (changes["enabled"].currentValue) {
        this.activate();
      } else {
        this.deactivate();
      }
    }

    if (changes.hasOwnProperty("resolution")) {
      // handle changing resolution
    }
  }

  getVideoElement(): HTMLVideoElement {
    return this.videoElement.nativeElement;
  }

  setMediaSource() {
    if (this.mediaStream) {
      let video = this.getVideoElement();
      video.srcObject = this.mediaStream;
      video.muted = true;
      this.muted = true;
      this.cameraPanelService.setMute(this.camera.metadata.name, this.muted);
      video.autoplay = true;
      //video.onloadeddata = this.genStatusHandler("active");
      video.onpause = this.genStatusHandler("loading");

      video.onended = () => {
        this.state = { kind: "new" };
      };

      video.ontimeupdate = () => {
        if (this.state.kind === "new") {
          let currentTime = this.getVideoElement().currentTime;
          console.log(`Current time! ${currentTime}`);
          this.state = { kind: "playing" };
        }
      };
    } else {
      //      let video = this.videoElement.nativeElement as HTMLVideoElement;
      //      video.pause();
      //      video.srcObject = null;
    }
  }

  activate() {
    this.subscription = this.webrtcService
      .subscribe(this.camera.metadata.name)
      .subscribe(
        (m) => {
          if (m !== this.mediaStream) {
            this.mediaStream = m;
          }
          this.setMediaSource();
        },
        (_err) => {
          //        this.mediaStream = undefined;
          //        this.setMediaSource();
        },
      );
  }

  deactivate() {
    if (this.subscription !== null) {
      this.subscription.unsubscribe();
    }
    if (this.videoElement !== undefined) {
      //      let video = this.videoElement.nativeElement as HTMLVideoElement;
      //      video.pause();
    }
  }

  toggleMute() {
    let muteValue = this.videoElement.nativeElement.muted;
    this.videoElement.nativeElement.muted = !muteValue;
    this.muted = !muteValue;
    this.cameraPanelService.setMute(this.camera.metadata.name, this.muted);
  }

  setStatus(_event: Event) {
    this.status = "active";
  }

  genStatusHandler(status: string) {
    return () => {
      this.onVideoStatusChange(status);
    };
  }

  onVideoStatusChange(status: string) {
    setTimeout(() => {
      this.status = status;
    }, 0);
  }
}
