import { NgModule } from '@angular/core';
import { Routes, RouterModule } from '@angular/router';
import { CameraPanelComponent } from './camera-panel/camera-panel.component';

const routes: Routes = [
  { path: 'cameras', component: CameraPanelComponent },
  { path: '', redirectTo: '/cameras', pathMatch: 'full' },
  { path: '**', redirectTo: '/cameras', pathMatch: 'full' },
];

@NgModule({
  imports: [RouterModule.forRoot(routes)],
  exports: [RouterModule]
})
export class AppRoutingModule { }
