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
  OnDestroy,
  OnInit,
  Output,
  SimpleChanges,
  ViewChild,
} from "@angular/core";
import { Subscription } from "rxjs";
import { Camera, CameraId } from "../camera";
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
export class CameraViewComponent implements OnInit, OnDestroy {
  @Input() camera!: Camera;
  @Input() selected = false;
  @Input() enabled = false;
  @Input() focused = false;
  @Input() muted = true;

  @Output() isVisible = new EventEmitter<boolean>();
  @Output() focusEvent = new EventEmitter<CameraId>();
  @Output() returnEvent = new EventEmitter<void>();

  @ViewChild("wrapperDiv") wrapperDiv!: ElementRef<HTMLDivElement>;

  @ViewChild("videoElement") videoElement!: ElementRef<HTMLVideoElement>;

  public status = "loading";

  private mediaStream?: MediaStream = undefined;
  private state: CameraViewStatus = { kind: "new" };
  private subscription: Subscription | null = null;
  private viewInitialized = false;

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
    this.viewInitialized = true;
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

    if (changes.hasOwnProperty("muted")) {
      this.applyMutedState();
    }

    if (changes.hasOwnProperty("resolution")) {
      // handle changing resolution
    }
  }

  ngOnDestroy() {
    this.deactivate();
    this.clearMediaSource();
  }

  getVideoElement(): HTMLVideoElement {
    return this.videoElement.nativeElement;
  }

  setMediaSource() {
    if (!this.viewInitialized) {
      return;
    }

    if (this.mediaStream) {
      let video = this.getVideoElement();
      video.srcObject = this.mediaStream;
      this.applyMutedState();
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
      this.clearMediaSource();
    }
  }

  activate() {
    if (this.subscription !== null) {
      return;
    }

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
      this.subscription = null;
    }
    if (this.videoElement !== undefined) {
      this.getVideoElement().pause();
    }
  }

  toggleMute() {
    this.muted = !this.muted;
    this.applyMutedState();
    this.cameraPanelService.setMute(this.camera.metadata.name, this.muted);
  }

  private applyMutedState() {
    if (this.videoElement !== undefined) {
      this.videoElement.nativeElement.muted = this.muted;
    }
  }

  private clearMediaSource() {
    if (this.videoElement !== undefined) {
      const video = this.getVideoElement();
      video.pause();
      video.srcObject = null;
    }
    this.mediaStream = undefined;
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
