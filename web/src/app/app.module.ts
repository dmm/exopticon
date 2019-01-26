import { BrowserModule } from '@angular/platform-browser';
import { NgModule } from '@angular/core';
import { HttpClientModule } from '@angular/common/http';

import { AppRoutingModule } from './app-routing.module';
import { AppComponent } from './app.component';
import { CameraPanelComponent } from './camera-panel/camera-panel.component';
import { CameraService } from './camera.service';

@NgModule({
  declarations: [
    AppComponent,
    CameraPanelComponent,
  ],
  imports: [
    BrowserModule,
    AppRoutingModule,
    HttpClientModule,
  ],
  providers: [CameraService],
  bootstrap: [AppComponent]
})
export class AppModule { }
