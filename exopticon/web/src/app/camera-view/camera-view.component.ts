import { SimpleChanges, Component, ChangeDetectorRef, ElementRef, OnInit, Output, EventEmitter, Input, NgZone } from '@angular/core';
import { Observable, Subscription } from 'rxjs';

import { Camera } from '../camera';
import { SubscriptionSubject, VideoService } from '../video.service';
import { CameraResolution, WsMessage } from '../frame-message';

@Component({
  selector: 'app-camera-view',
  templateUrl: './camera-view.component.html',
  styleUrls: ['./camera-view.component.css']
})
export class CameraViewComponent implements OnInit {
  @Input() camera: Camera;
  @Input() selected: boolean;
  @Input() enabled: boolean;
  @Input() videoService: VideoService;
  @Input() resolution: CameraResolution;

  @Output() isVisible = new EventEmitter<boolean>();

  public status: string;

  private videoSubject: SubscriptionSubject;
  public frameService?: Observable<WsMessage>;

  constructor(private changeRef: ChangeDetectorRef) {
  }

  ngOnInit() {
    if (this.enabled) {
      this.activate();
    }
  }

  ngOnChanges(changes: SimpleChanges) {
    if (changes.hasOwnProperty('enabled')) {
      if (changes['enabled'].currentValue) {
        this.activate()
      } else {
        this.deactivate();
      }
    }

    if (changes.hasOwnProperty('resolution')) {
      // handle changing resolution
    }
  }

  activate() {
    this.videoSubject = {
      kind: 'camera',
      cameraId: this.camera.id,
      resolution: this.resolution,
    };
    this.frameService = this.videoService.getObservable(this.videoSubject);
  }

  deactivate() {
    this.frameService = undefined;
  }

  onVideoStatusChange(status: string) {
    setTimeout(() => {
      this.status = status;
    }, 0);
  }

  onInViewportChange(inViewport: boolean) {
    this.isVisible.emit(inViewport);
  }
}
