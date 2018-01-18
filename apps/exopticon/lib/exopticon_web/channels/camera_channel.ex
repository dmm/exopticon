defmodule ExopticonWeb.CameraChannel do
  use ExopticonWeb, :channel

  import Logger

  intercept(["jpg"])

  def join("camera:lobby", payload, socket) do
    if authorized?(payload) do
      {:ok, socket}
    else
      {:error, %{reason: "unauthorized"}}
    end
  end

  def join("camera:stream", _params, socket) do
    send(self, :after_join)
    {:ok, socket}
  end

  def handle_info(:after_join, socket) do
    push(socket, "subscribe", %{})
    {:noreply, socket}
  end

  def handle_in("watch" <> camera_id = topic, _payload, socket) do
    watch_cameras = socket.assigns[:watch_camera]
    new_watch = Map.put_new(watch_cameras, String.to_integer(camera_id), 1)
    socket = assign(socket, :watch_camera, new_watch)
    {:noreply, socket}
  end

  def handle_in("close" <> camera_id, _payload, socket) do
    watch_cameras = socket.assigns[:watch_camera]
    new_watch = Map.delete(watch_cameras, camera_id)
    socket = assign(socket, :watch_cameras, new_watch)
    {:noreply, socket}
  end

  # Channels can be used in a request/response fashion
  # by sending replies to requests from the client
  def handle_in("ping", payload, socket) do
    {:reply, {:ok, payload}, socket}
  end

  def handle_in("ack", _payload, socket) do
    cur_live = socket.assigns[:cur_live]
    socket = assign(socket, :cur_live, cur_live - 1)
    {:noreply, socket}
  end

  # It is also common to receive messages from the client and
  # broadcast to everyone in the current topic (camera:lobby).
  def handle_in("shout", payload, socket) do
    broadcast(socket, "shout", payload)
    {:noreply, socket}
  end

  def adjust_max_live(true, cur_live, max_live) when cur_live < max_live do
    max_live
  end

  def adjust_max_live(true, cur_live, max_live) when cur_live >= max_live do
    Enum.max([max_live - 1, 1])
  end

  def adjust_max_live(true, cur_live, max_live) when cur_live < max_live and cur_live == 0 do
    Enum.min([max_live + 1, 20])
  end

  def adjust_max_live(false, _cur_live, max_live) do
    max_live
  end

  def adjust_cur_live(true, cur_live, max_live) when cur_live < max_live do
    cur_live + 1
  end

  def adjust_cur_live(true, cur_live, max_live) when cur_live >= max_live do
    cur_live
  end

  def adjust_cur_live(false, cur_live, max_live) do
    cur_live
  end

  def handle_out("jpg", params, socket) do
    cur_live = socket.assigns[:cur_live]
    max_live = socket.assigns[:max_live]
    camera_id = params[:cameraId]
    watch_camera = socket.assigns[:watch_camera]

    camera_active = Map.has_key?(watch_camera, camera_id)
    frame_active = camera_active and cur_live < max_live

    new_max_live = adjust_max_live(camera_active, cur_live, max_live)
    new_cur_live = adjust_cur_live(camera_active, cur_live, max_live)

    socket = assign(socket, :max_live, new_max_live)
    socket = assign(socket, :cur_live, new_cur_live)

    if frame_active do
      push(socket, "jpg" <> Integer.to_string(camera_id), params)
    end

    {:noreply, socket}
  end

  # Add authorization logic here as required.
  defp authorized?(_payload) do
    true
  end
end
