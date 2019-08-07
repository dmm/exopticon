import { Component, ChangeDetectorRef, OnInit, Input, NgZone } from '@angular/core';
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
  pageVisible: boolean;
  private cameraVisibility: Map<number, boolean>;

  constructor(private cameraService: CameraService,
    public videoService: VideoService,
    private cdr: ChangeDetectorRef,
    private ngZone: NgZone) {
    this.cameraVisibility = new Map<number, boolean>();
  }

  getCameras(): void {
    this.cameraService
      .getCameras()
      .subscribe(
        cameras => {
          this.cameras = cameras.filter(c => c.enabled);
        }
        ,
        () => {
          window.location.pathname = '/login';
        }
      )
  }

  ngOnInit() {
    this.pageVisible = true;
    this.getCameras();
    this.videoService.connect();
  }

  @OnPageVisible()
  onPageVisible() {
    this.ngZone.run(() => {
      this.pageVisible = true;
    });
  }

  @OnPageHidden()
  onPageHidden() {
    this.ngZone.run(() => {
      this.pageVisible = false;
    });
  }


  selectCameraView(cameraIndex: number) {
    if (cameraIndex !== this.overlayDisabledId) {
      this.selectedCameraId = cameraIndex;
    }
  }

  deselectCameraView() {
    this.selectedCameraId = null;
  }

  onTouchStart(cameraIndex: number) {
    if (this.selectedCameraId === cameraIndex) {
      this.selectedCameraId = null;
      this.overlayDisabledId = cameraIndex;
    } else {
      this.selectedCameraId = cameraIndex;
      this.overlayDisabledId = null;
    }
  }

  updateCameraViewVisibility(cameraId: number, visible: boolean) {
    this.cameraVisibility.set(cameraId, visible);
  }

}
