import { NgModule } from "@angular/core";
import { RouterModule, Routes } from "@angular/router";
import { AlertViewComponent } from "./alert-view/alert-view.component";
import { AnalysisPanelComponent } from "./analysis-panel/analysis-panel.component";
import { CameraPanelComponent } from "./camera-panel/camera-panel.component";
import { PlaybackViewComponent } from "./playback-view/playback-view.component";

const routes: Routes = [
  { path: "cameras", component: CameraPanelComponent },
  { path: "analysis_engine/:id", component: AnalysisPanelComponent },
  { path: "cameras/:id/playback", component: PlaybackViewComponent },
  { path: "alerts/:id", component: AlertViewComponent },
  { path: "", redirectTo: "/cameras", pathMatch: "full" },
  { path: "**", redirectTo: "/cameras", pathMatch: "full" },
];

@NgModule({
  imports: [RouterModule.forRoot(routes, { relativeLinkResolution: 'legacy' })],
  exports: [RouterModule],
})
export class AppRoutingModule {}
