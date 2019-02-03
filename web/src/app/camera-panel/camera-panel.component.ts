import { Component, OnInit, Input } from '@angular/core';

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

  constructor(private cameraService: CameraService, private videoService: VideoService) { }

  getCameras(): void {
    this.cameraService
      .getCameras()
      .subscribe(
        cameras => (this.cameras = cameras.filter(c => c.enabled)),
        error => (this.error = error)
      )
  }

  ngOnInit() {
    this.getCameras();
    this.videoService.connect();
  }

  selectCameraView(i: number) {
    this.selectedCameraId = i;
  }

  deselectCameraView() {
    this.selectedCameraId = null;
  }
}
