import { ComponentFixture, TestBed, waitForAsync } from "@angular/core/testing";
import { VideoViewComponent } from "./video-view.component";

describe("VideoViewComponent", () => {
  let component: VideoViewComponent;
  let fixture: ComponentFixture<VideoViewComponent>;

  beforeEach(waitForAsync(() => {
    TestBed.configureTestingModule({
      declarations: [VideoViewComponent],
    }).compileComponents();
  }));

  beforeEach(() => {
    fixture = TestBed.createComponent(VideoViewComponent);
    component = fixture.componentInstance;
    fixture.detectChanges();
  });

  it("should create", () => {
    expect(component).toBeTruthy();
  });
});
