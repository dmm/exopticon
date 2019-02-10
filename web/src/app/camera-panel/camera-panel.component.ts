import { Component, ChangeDetectorRef, OnInit, Input } from '@angular/core';
import { OnPageVisible, OnPageHidden } from 'angular-page-visibility';

import { Camera } from '../camera';
import { CameraService } from '../camera.service';
import { VideoService } from '../video.service';

@Component({
  selector: 'app-camera-panel',
  templateUrl: './camera-panel.component.html',
  styleUrls: ['./camera-panel.component.css']
})
export class CameraPanelComponent implements OnInit {
  cameras: Camera[];
  error: any;
  selectedCameraId?: number;
  overlayDisabledId?: number;
  inViewCameraIds: boolean[];
  pageVisible: boolean;

  constructor(private cameraService: CameraService,
    private videoService: VideoService,
    private cdr: ChangeDetectorRef) {
    this.inViewCameraIds = [];

  }

  getCameras(): void {
    this.cameraService
      .getCameras()
      .subscribe(
        cameras => {
          this.cameras = cameras.filter(c => c.enabled);
          this.cameras.forEach((c) => {
            this.inViewCameraIds.push(false);
          });
        }
        ,
        error => (this.error = error)
      )
  }

  ngOnInit() {
    this.pageVisible = true;
    this.getCameras();
    this.videoService.connect();
  }

  selectCameraView(i: number) {
    if (i !== this.overlayDisabledId) {
      this.selectedCameraId = i;
    }
  }

  deselectCameraView() {
    this.selectedCameraId = null;
  }

  onTouchStart(cameraId: number) {
    if (this.selectedCameraId === cameraId) {
      this.selectedCameraId = null;
      this.overlayDisabledId = cameraId;
    } else {
      this.selectedCameraId = cameraId;
      this.overlayDisabledId = null;
    }
  }

  onInViewportChange(inViewport: boolean, cameraId: number) {
    this.inViewCameraIds[cameraId] = inViewport;
  }

  @OnPageVisible()
  onPageVisible() {
    this.pageVisible = true;
    this.cdr.detectChanges();
  }

  @OnPageHidden()
  onPageHidden() {
    this.pageVisible = false;
    this.cdr.detectChanges();
  }

}
