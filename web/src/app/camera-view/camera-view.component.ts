import { Component, ChangeDetectorRef, ElementRef, OnInit, Input, NgZone } from '@angular/core';
import { OnPageVisible, OnPageHidden } from 'angular-page-visibility';
import { Observable, Subscription } from 'rxjs';

import { Camera } from '../camera';
import { SubscriptionSubject, VideoService } from '../video.service';
import { CameraResolution } from '../frame-message';

@Component({
  selector: 'app-camera-view',
  templateUrl: './camera-view.component.html',
  styleUrls: ['./camera-view.component.css']
})
export class CameraViewComponent implements OnInit {
  @Input() camera: Camera;
  @Input() selected: boolean;
  @Input() videoService: VideoService;

  public status: string;

  public videoSubject: SubscriptionSubject;
  constructor() {
  }

  ngOnInit() {
    this.videoSubject = {
      kind: 'camera',
      cameraId: this.camera.id,
      resolution: CameraResolution.Sd,
    };
  }

  onVideoStatusChange(status: string) {
    this.status = status;
  }
}