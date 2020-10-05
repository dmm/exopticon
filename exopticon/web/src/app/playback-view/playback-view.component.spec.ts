import { async, ComponentFixture, TestBed } from "@angular/core/testing";
import { PlaybackViewComponent } from "./playback-view.component";

describe("PlaybackViewComponent", () => {
  let component: PlaybackViewComponent;
  let fixture: ComponentFixture<PlaybackViewComponent>;

  beforeEach(async(() => {
    TestBed.configureTestingModule({
      declarations: [PlaybackViewComponent],
    }).compileComponents();
  }));

  beforeEach(() => {
    fixture = TestBed.createComponent(PlaybackViewComponent);
    component = fixture.componentInstance;
    fixture.detectChanges();
  });

  it("should create", () => {
    expect(component).toBeTruthy();
  });
});
