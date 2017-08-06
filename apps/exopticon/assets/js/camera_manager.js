
Array.prototype.diff = function(a) {
    return this.filter(function(i) {
        return a.indexOf(i) < 0;
    });
};

function renderFrame(img, imageArrayBuffer) {
    var blob  = new Blob([imageArrayBuffer],{type: "image/jpeg"});
    img.onload = function (e) {
        window.URL.revokeObjectURL(img.src);
        img = null;
    };

    img.onerror = img.onabort = function () {
        console.log('error loading image!');
        img = null;
    };
    img.src = window.URL.createObjectURL(blob);
}

var camera = function(id, name) {
    this.id = id;
    this.name = name;

    return this;
};

var CameraManager = function(socket) {
    this.cameras = new Map();
    this.channels = new Map();
    this.socket = socket;
};

CameraManager.prototype = {
    startNewCamera: function(newCamera) {
        let channel = this.socket.channel("camera:" + newCamera.id);
        newCamera.channel = channel;
        let img = document.createElement("img");
        img.id = 'camera' + newCamera.id;
        let videoContainer = document.getElementById("cameras");
        videoContainer.appendChild(img);
        channel.on("jpg", function(data) {
            renderFrame(img, data.frameJpeg);
        });
        channel.join();
        this.cameras.set(newCamera.id, newCamera);
    },

    removeCamera: function(oldCamera) {
        var channel = this.oldCamera.channel;
        channel.leave();
        this.cameras = _.filter(this.cameras, function(c) {
            return c.id != oldCamera.id;
        });
        var element = document.getElementById('camera'+oldCamera.id);
        element.outerHTML = '';
        this.cameras.delete(oldCamera.id);
    },

    updateCameras: function(allCameras) {
        let curMap = new Map();
        let self = this;

        allCameras.forEach(function(c) {
            curMap.set(c.id, c);
            if (!self.cameras.has(c.id)) {
                self.startNewCamera(c);
            }
        });

        for (var [key, value] of this.cameras) {
            if (!curMap.has(key)) {
                // Camera has been removed
                self.removeCamera(value);
            }
        }
    }
};

export default CameraManager;
