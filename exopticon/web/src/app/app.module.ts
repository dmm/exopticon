import { HttpClientModule } from "@angular/common/http";
import { NgModule } from "@angular/core";
import { BrowserModule } from "@angular/platform-browser";
import { InViewportModule } from "@thisissoon/angular-inviewport";
import { AlertViewComponent } from "./alert-view/alert-view.component";
import { AnalysisPanelComponent } from "./analysis-panel/analysis-panel.component";
import { AppRoutingModule } from "./app-routing.module";
import { AppComponent } from "./app.component";
import { CameraOverlayComponent } from "./camera-overlay/camera-overlay.component";
import { CameraPanelComponent } from "./camera-panel/camera-panel.component";
import { CameraStatusOverlayComponent } from "./camera-status-overlay/camera-status-overlay.component";
import { CameraViewComponent } from "./camera-view/camera-view.component";
import { CameraService } from "./camera.service";
import { PlaybackViewComponent } from "./playback-view/playback-view.component";
import { VideoViewComponent } from "./video-view/video-view.component";

@NgModule({
  declarations: [
    AppComponent,
    CameraPanelComponent,
    CameraViewComponent,
    CameraOverlayComponent,
    CameraStatusOverlayComponent,
    AnalysisPanelComponent,
    VideoViewComponent,
    PlaybackViewComponent,
    AlertViewComponent,
  ],
  imports: [
    BrowserModule,
    AppRoutingModule,
    InViewportModule,
    HttpClientModule,
  ],
  providers: [CameraService],
  bootstrap: [AppComponent],
})
export class AppModule {}
