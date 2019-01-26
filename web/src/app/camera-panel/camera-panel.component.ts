import { Component, OnInit, Input } from '@angular/core';

import { Camera } from '../camera';
import { CameraService } from '../camera.service';

@Component({
  selector: 'app-camera-panel',
  templateUrl: './camera-panel.component.html',
  styleUrls: ['./camera-panel.component.css']
})
export class CameraPanelComponent implements OnInit {
  cameras: Camera[];
  error: any;

  constructor(private cameraService: CameraService) { }

  getCameras(): void {
    this.cameraService
      .getCameras()
      .subscribe(
        cameras => (this.cameras = cameras),
        error => (this.error = error)
      )
  }

  ngOnInit() {
    this.getCameras();
  }

}
