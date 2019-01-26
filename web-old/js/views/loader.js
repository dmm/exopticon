
import MainView from './main';
import PageIndexView from './page_index_view.js';
import FileBrowseView from './file/browse';
import FileShowView from './file/show';
import CameraPlaybackView from './camera/playback';

// Collection of specific view modules
const views = {
  PageIndexView,
  FileShowView,
  CameraPlaybackView,
  FileBrowseView,
};
/**
 * returns the implementation behind the supplied view name
 * @param {string} viewName
 * @return {function}
 */
export default function loadView(viewName) {
  return views[viewName] || MainView;
}
