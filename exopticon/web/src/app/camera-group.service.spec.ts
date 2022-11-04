import { TestBed } from "@angular/core/testing";

import { CameraGroupService } from "./camera-group.service";

describe("CameraGroupService", () => {
  let service: CameraGroupService;

  beforeEach(() => {
    TestBed.configureTestingModule({});
    service = TestBed.inject(CameraGroupService);
  });

  it("should be created", () => {
    expect(service).toBeTruthy();
  });
});
