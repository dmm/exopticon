import { NgModule } from '@angular/core';
import { Routes, RouterModule } from '@angular/router';
import { AnalysisPanelComponent } from './analysis-panel/analysis-panel.component';
import { CameraPanelComponent } from './camera-panel/camera-panel.component';
import { PlaybackViewComponent } from './playback-view/playback-view.component';

const routes: Routes = [
  { path: 'cameras', component: CameraPanelComponent },
  { path: 'analysis_engine/:id', component: AnalysisPanelComponent },
  { path: 'cameras/:id/playback', component: PlaybackViewComponent },
  { path: '', redirectTo: '/cameras', pathMatch: 'full' },
  { path: '**', redirectTo: '/cameras', pathMatch: 'full' },
];

@NgModule({
  imports: [RouterModule.forRoot(routes)],
  exports: [RouterModule]
})
export class AppRoutingModule { }
