import { Component, ChangeDetectorRef, HostListener, OnInit, Input, NgZone } from '@angular/core';
import { ActivatedRoute, Router, Params } from '@angular/router';
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
  enabledCameras: Camera[];
  enabledCamerasOffset: number = 0;
  error: any;
  selectedCameraId?: number;
  overlayDisabledId?: number;
  pageVisible: boolean;
  private cameraVisibility: Map<number, boolean>;
  private columns: number;
  private rows: number;

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
    this.route.paramMap.subscribe(params => {
      if (params.has('columns')) {
        this.columns = parseInt(params.get('columns'), 10);
      } else {
        this.columns = 1;
      }
      if (params.has('rows')) {
        this.rows = parseInt(params.get('rows'), 10);
      } else {
        this.rows = -1;
      }

      if (params.has('cameraOffset')) {
        this.enabledCamerasOffset = parseInt(params.get('cameraOffset'), 10) || 0;
      } else {
        this.enabledCamerasOffset = 0;
      }

      this.enableCameras();
    });

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

  @HostListener('window:keyup', ['$event'])
  KeyEvent(event: KeyboardEvent) {
    console.log(event.keyCode);
    let offset = this.enabledCamerasOffset;
    if (event.keyCode === 37) {
      offset--;
    }
    if (event.keyCode === 39) {
      offset++;
    }

    this.router.navigate(['./', this.merge({ 'cameraOffset': offset }, this.route.snapshot.params)],
      {
        queryParamsHandling: 'preserve',
        relativeTo: this.route
      });
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
    let count = this.columns === -1 ? this.cameras.length : this.columns * this.rows;
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

}
