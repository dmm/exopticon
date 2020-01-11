import { Component, ChangeDetectorRef, HostListener, OnInit, Input, NgZone } from '@angular/core';
import { ActivatedRoute, Router, Params } from '@angular/router';
import { Camera } from '../camera';
import { CameraService, PtzDirection } from '../camera.service';
import { VideoService } from '../video.service';
import { CameraResolution } from '../frame-message';

@Component({
  selector: 'app-camera-panel',
  templateUrl: './camera-panel.component.html',
  styleUrls: ['./camera-panel.component.css']
})
export class CameraPanelComponent implements OnInit {
  cameras: Camera[];
  enabledCameras: Camera[];
  enabledCamerasOffset: number = 0;
  error: any;
  selectedCameraId?: number;
  overlayDisabledId?: number;
  pageVisible: boolean;
  private cameraVisibility: Map<number, boolean>;
  private columns: number;
  private rows: number;
  private resolution: CameraResolution;

  constructor(private cameraService: CameraService,
    public videoService: VideoService,
    private cdr: ChangeDetectorRef,
    private route: ActivatedRoute,
    private router: Router,
    private ngZone: NgZone) {
    this.cameras = [];
    this.enabledCameras = [];
    this.cameraVisibility = new Map<number, boolean>();
    this.columns = 1;
    this.rows = -1;
    this.resolution = CameraResolution.Sd;
  }

  getCameras(): void {
    this.cameraService
      .getCameras()
      .subscribe(
        cameras => {
          this.cameras = cameras.filter(c => c.enabled);
          this.enableCameras();
        }
        ,
        () => {
          window.location.pathname = '/login';
        }
      )
  }

  ngOnInit() {
    this.pageVisible = true;

    this.route.paramMap.subscribe(params => {
      if (params.has('cols')) {
        this.columns = parseInt(params.get('cols'), 10);
      } else {
        this.columns = 1;
      }
      if (params.has('rows')) {
        this.rows = parseInt(params.get('rows'), 10);
      } else {
        this.rows = -1;
      }

      if (params.has('offset')) {
        this.enabledCamerasOffset = parseInt(params.get('offset'), 10) || 0;
      } else {
        this.enabledCamerasOffset = 0;
      }

      if (params.has('res')) {
        if (params.get('res').toLowerCase() === 'hd') {
          this.resolution = CameraResolution.Hd;
        } else {
          this.resolution = CameraResolution.Sd;
        }
      }
      this.enableCameras();
    });

    this.getCameras();
    this.videoService.connect();
  }

  @HostListener('document:visibilitychange', ['$event'])
  onVisibilityChange() {
    this.ngZone.run(() => {
      this.pageVisible = !document['hidden'];
    });
  }

  @HostListener('window:keyup', ['$event'])
  KeyEvent(event: KeyboardEvent) {
    console.log('keycode: ' + event.keyCode);
    let offset = this.enabledCamerasOffset;
    let cameraId = this.getKeyboardControlCameraId();
    console.log('keyboard control camera id: ' + cameraId);
    switch (event.keyCode) {
      case 78:
        // 'n'
        offset++;
        break;
      case 80:
        // 'p'
        offset--;
        break;
      case 65:
        // 'a'
        if (cameraId)
          this.cameraService.ptz(cameraId, PtzDirection.left);
        break;
      case 68:
        // 'd'
        if (cameraId)
          this.cameraService.ptz(cameraId, PtzDirection.right);
        break;
      case 87:
        // 'w'
        if (cameraId)
          this.cameraService.ptz(cameraId, PtzDirection.up);
        break;
      case 83:
        // 's'
        if (cameraId)
          this.cameraService.ptz(cameraId, PtzDirection.down);
        break;
    }

    if (offset !== this.enabledCamerasOffset) {
      this.router.navigate(['./', this.merge({ 'offset': offset }, this.route.snapshot.params)],
        {
          queryParamsHandling: 'preserve',
          relativeTo: this.route
        });
    }
  }

  merge(newParams: any, oldParams: any): any {
    let params = Object.assign({}, oldParams);

    for (var prop in newParams) {
      if (newParams.hasOwnProperty(prop)) {
        let newValue = newParams[prop];
        if (newValue === null) {
          delete params[prop];
        } else {
          params[prop] = newValue;
        }
      }
    }

    return params;
  }

  rotateArray(arr: Array<any>, length: number): Array<any> {
    if (arr.length === 0) return [];
    arr = arr.slice();

    if (length > 0) {
      for (let i = 0; i < length; i++) {
        arr.unshift(arr.pop());
      }
    } else {
      for (let i = 0; i < Math.abs(length); i++) {
        arr.push(arr.shift());
      }
    }

    return arr;
  }

  enableCameras() {
    let count = this.rows === -1 ? this.cameras.length : this.columns * this.rows;
    this.enabledCameras = this.rotateArray(this.cameras, this.enabledCamerasOffset).slice(0, count);
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

  getKeyboardControlCameraId(): number {
    if (this.enabledCameras.length === 1) {
      return this.enabledCameras[0].id;
    } else if (this.selectedCameraId) {
      return this.selectedCameraId;
    }

    return -1;
  }

}
