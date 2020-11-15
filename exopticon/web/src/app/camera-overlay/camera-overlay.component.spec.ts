import { ComponentFixture, TestBed, waitForAsync } from "@angular/core/testing";
import { CameraOverlayComponent } from "./camera-overlay.component";

describe("CameraOverlayComponent", () => {
  let component: CameraOverlayComponent;
  let fixture: ComponentFixture<CameraOverlayComponent>;

  beforeEach(waitForAsync(() => {
    TestBed.configureTestingModule({
      declarations: [CameraOverlayComponent],
    }).compileComponents();
  }));

  beforeEach(() => {
    fixture = TestBed.createComponent(CameraOverlayComponent);
    component = fixture.componentInstance;
    fixture.detectChanges();
  });

  it("should create", () => {
    expect(component).toBeTruthy();
  });
});
