const merge = require('webpack-merge');
const CompressionPlugin = require('compression-webpack-plugin');
const UglifyJsPlugin = require('uglifyjs-webpack-plugin');

const common = require('./webpack.common.js');

module.exports = merge(common, {
    plugins: [
        new CompressionPlugin({
            test: /\.js/,
        }),
        new UglifyJsPlugin({
            parallel: true,
            test: /\.js($|\?)/i,
            sourceMap: true,
        }),
    ],
});
