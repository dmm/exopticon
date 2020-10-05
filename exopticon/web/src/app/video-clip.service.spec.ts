import { TestBed } from "@angular/core/testing";
import { VideoClipService } from "./video-clip.service";

describe("VideoClipService", () => {
  let service: VideoClipService;

  beforeEach(() => {
    TestBed.configureTestingModule({});
    service = TestBed.inject(VideoClipService);
  });

  it("should be created", () => {
    expect(service).toBeTruthy();
  });
});
