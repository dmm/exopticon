
import MainView    from './main';
import PageIndexView from './page_index_view.js';
import FileShowView from './file/show';
import CameraPlaybackView from './camera/playback';

// Collection of specific view modules
const views = {
    PageIndexView,
    FileShowView,
    CameraPlaybackView
};

export default function loadView(viewName) {
  return views[viewName] || MainView;
}
