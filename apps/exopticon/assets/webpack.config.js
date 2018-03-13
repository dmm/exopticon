const ExtractTextPlugin = require("extract-text-webpack-plugin");
const CompressionPlugin = require("compression-webpack-plugin");
const CopyWebpackPlugin = require("copy-webpack-plugin");
const UglifyJsPlugin = require('uglifyjs-webpack-plugin');
var webpack = require('webpack');

module.exports = {
  entry: ['./js/app.js', './css/app.css'],
  devtool: 'source-map',
  output: {
    path: __dirname + '/../priv/static/',
    filename: 'js/app.js'
  },
  plugins: [

    new CompressionPlugin({
      test: /\.js/
    }),

    new ExtractTextPlugin("css/app.css"),
    new CopyWebpackPlugin([{ from: "./static/" }]),
    /*
    new UglifyJsPlugin({
      parallel: true,
      test: /\.js($|\?)/i,
      sourceMap: true,
    }),
    */
  ],
  module: {
    rules: [
      {
        test: /\.js$/,
        exclude: [/node_modules/, /vendor/],
        loader: 'babel-loader',
        include: __dirname,
        query: {
          presets: ['env', 'react']
        }
      },
      {
        test: /\.css$/,
        use: ExtractTextPlugin.extract({
          fallback: "style-loader",
          use: "css-loader"
        })
      }
    ]
  }
};
