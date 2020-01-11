import { BrowserModule } from '@angular/platform-browser';
import { NgModule } from '@angular/core';
import { HttpClientModule } from '@angular/common/http';
import { InViewportModule } from '@thisissoon/angular-inviewport';

import { AppRoutingModule } from './app-routing.module';
import { AppComponent } from './app.component';
import { CameraPanelComponent } from './camera-panel/camera-panel.component';
import { CameraService } from './camera.service';
import { CameraViewComponent } from './camera-view/camera-view.component';
import { CameraOverlayComponent } from './camera-overlay/camera-overlay.component';
import { CameraStatusOverlayComponent } from './camera-status-overlay/camera-status-overlay.component';
import { AnalysisPanelComponent } from './analysis-panel/analysis-panel.component';
import { VideoViewComponent } from './video-view/video-view.component';
import { PlaybackViewComponent } from './playback-view/playback-view.component';

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
  ],
  imports: [
    BrowserModule,
    AppRoutingModule,
    InViewportModule,
    HttpClientModule,
  ],
  providers: [CameraService],
  bootstrap: [AppComponent]
})
export class AppModule { }
