import {
  ChangeDetectorRef,
  Component,
  EventEmitter,
  Input,
  OnInit,
  Output,
  SimpleChanges,
  ViewChild,
  ElementRef,
} from "@angular/core";
import { Observable } from "rxjs";
import { Camera } from "../camera";
import { CameraResolution, WsMessage } from "../frame-message";
import { SubscriptionSubject, VideoService } from "../video.service";
import { ElementVisibleService } from "../element-visible.service";

@Component({
  selector: "app-camera-view",
  templateUrl: "./camera-view.component.html",
  styleUrls: ["./camera-view.component.css"],
})
export class CameraViewComponent implements OnInit {
  @Input() camera: Camera;
  @Input() selected: boolean;
  @Input() enabled: boolean;
  @Input() videoService: VideoService;
  @Input() resolution: CameraResolution;

  @Output() isVisible = new EventEmitter<boolean>();

  @ViewChild("wrapperDiv") wrapperDiv: ElementRef;

  public status: string;

  private videoSubject: SubscriptionSubject;
  public frameService?: Observable<WsMessage>;

  constructor(
    private changeRef: ChangeDetectorRef,
    private visibilityService: ElementVisibleService
  ) {}

  ngOnInit() {
    if (this.enabled) {
      this.activate();
    }
  }

  ngAfterViewInit() {
    this.visibilityService
      .elementVisible(this.wrapperDiv)
      .subscribe((visible) => {
        this.isVisible.emit(visible);
      });
  }

  ngOnChanges(changes: SimpleChanges) {
    if (changes.hasOwnProperty("enabled")) {
      if (changes["enabled"].currentValue) {
        this.activate();
      } else {
        this.deactivate();
      }
    }

    if (changes.hasOwnProperty("resolution")) {
      // handle changing resolution
    }
  }

  activate() {
    this.videoSubject = {
      kind: "camera",
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

  onInViewportChange(inViewport: boolean) {}
}
