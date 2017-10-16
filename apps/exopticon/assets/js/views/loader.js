
import MainView    from './main';
import PageIndexView from './page_index_view.js';
import FileShowView from './file/show';

// Collection of specific view modules
const views = {
    PageIndexView,
    FileShowView
};

export default function loadView(viewName) {
  return views[viewName] || MainView;
}
