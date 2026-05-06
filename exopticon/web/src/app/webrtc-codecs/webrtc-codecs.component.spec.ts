import { ComponentFixture, TestBed } from "@angular/core/testing";

import { WebrtcCodecsComponent } from "./webrtc-codecs.component";

describe("WebrtcCodecsComponent", () => {
  let component: WebrtcCodecsComponent;
  let fixture: ComponentFixture<WebrtcCodecsComponent>;

  beforeEach(async () => {
    await TestBed.configureTestingModule({
      imports: [WebrtcCodecsComponent],
    }).compileComponents();

    fixture = TestBed.createComponent(WebrtcCodecsComponent);
    component = fixture.componentInstance;
    await fixture.whenStable();
  });

  it("should create", () => {
    expect(component).toBeTruthy();
  });
});
