import { Component, ElementRef, OnInit, Input } from '@angular/core';
import { OnPageVisible, OnPageHidden } from 'angular-page-visibility';
import { Observable, Subscription } from 'rxjs';

import { Camera } from '../camera';
import { VideoService } from '../video.service';
import { FrameMessage } from '../frame-message';

@Component({
  selector: 'app-camera-view',
  templateUrl: './camera-view.component.html',
  styleUrls: ['./camera-view.component.css']
})
export class CameraViewComponent implements OnInit {
  @Input() camera: Camera;
  @Input() selected: boolean;
  @Input() videoService: VideoService;

  private frameService?: Observable<FrameMessage>;
  private subscription?: Subscription;
  private img: HTMLImageElement;
  private inViewport: boolean;

  constructor(private elementRef: ElementRef) { }

  ngAfterContentInit() {
    this.img = this.elementRef.nativeElement.querySelector('img');
  }

  ngOnInit() {
    this.activate();
  }

  @OnPageVisible()
  activate() {
    if (this.inViewport) {
      this.deactivate();
      this.frameService = this.videoService.getObservable(this.camera.id, 'SD');
      this.subscription = this.frameService.subscribe((message) => {

        if (this.img.complete) {
          this.img.onerror = () => { console.log("error!"); };
          this.img.src = `data:image/jpeg;base64, ${message.jpeg}`;
        }
      });
    }
  }

  @OnPageHidden()
  deactivate() {
    if (this.subscription) {
      this.subscription.unsubscribe();
      this.subscription = null;
      this.frameService = null;
    }
  }

  onInViewportChange(inViewport: boolean) {
    console.log(`Viewport change for camera: ${this.camera.id} ${inViewport}`);
    this.inViewport = inViewport;
    if (inViewport) {
      this.activate();
    } else {
      this.deactivate();
    }
  }

}


