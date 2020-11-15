import { ComponentFixture, TestBed, waitForAsync } from "@angular/core/testing";
import { CameraStatusOverlayComponent } from "./camera-status-overlay.component";

describe("CameraStatusOverlayComponent", () => {
  let component: CameraStatusOverlayComponent;
  let fixture: ComponentFixture<CameraStatusOverlayComponent>;

  beforeEach(waitForAsync(() => {
    TestBed.configureTestingModule({
      declarations: [CameraStatusOverlayComponent],
    }).compileComponents();
  }));

  beforeEach(() => {
    fixture = TestBed.createComponent(CameraStatusOverlayComponent);
    component = fixture.componentInstance;
    fixture.detectChanges();
  });

  it("should create", () => {
    expect(component).toBeTruthy();
  });
});
