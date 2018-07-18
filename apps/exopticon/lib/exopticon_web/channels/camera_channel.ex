# This file is a part of Exopticon, a free video surveillance tool. Visit
# https://exopticon.org for more information.
#
# Copyright (C) 2018 David Matthew Mattli
#
# This program is free software: you can redistribute it and/or modify
# it under the terms of the GNU Affero General Public License as published by
# the Free Software Foundation, either version 3 of the License, or
# (at your option) any later version.
#
# This program is distributed in the hope that it will be useful,
# but WITHOUT ANY WARRANTY; without even the implied warranty of
# MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
# GNU Affero General Public License for more details.
# You should have received a copy of the GNU Affero General Public License
# along with this program.  If not, see <https://www.gnu.org/licenses/>.

defmodule ExopticonWeb.CameraChannel do
  @moduledoc """
  Provides channel implementation of live camera streams
  """
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
    send(self(), :after_join)
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
    Logger.debug("closing socket")
    watch_cameras = socket.assigns[:watch_camera]
    new_watch = Map.delete(watch_cameras, String.to_integer(camera_id))
    socket = assign(socket, :watch_camera, new_watch)
    {:noreply, socket}
  end

  def handle_in("hdon" <> camera_id = topic, _payload, socket) do
    hd_cameras = socket.assigns[:hd_cameras]
    new_hd_cameras = Map.put_new(hd_cameras, String.to_integer(camera_id), 1)
    socket = assign(socket, :hd_cameras, new_hd_cameras)
    {:noreply, socket}
  end

  def handle_in("hdoff" <> camera_id = topic, _payload, socket) do
    hd_cameras = socket.assigns[:hd_cameras]
    new_hd_cameras = Map.delete(hd_cameras, String.to_integer(camera_id))
    socket = assign(socket, :hd_cameras, new_hd_cameras)
    {:noreply, socket}
  end

  # Channels can be used in a request/response fashion
  # by sending replies to requests from the client
  def handle_in("ping", payload, socket) do
    {:reply, {:ok, payload}, socket}
  end

  def handle_in("ack", payload, socket) do
    cur_live = socket.assigns[:cur_live]
    max_live = socket.assigns[:max_live]
    cur_time = System.monotonic_time(:milliseconds)
    %{"ts" => ts} = payload
    {ts_int, _} = Integer.parse(ts)
    old_rtt = socket.assigns[:rtt]
    new_rtt = cur_time - ts_int
    rtt = (new_rtt + old_rtt) / 2

    max_live =
      if new_rtt > 2 * old_rtt do
        Enum.max([div(max_live, 2), 1])
      else
        Enum.min([max_live + 1, 10])
      end

    socket = assign(socket, :max_live, max_live)
    socket = assign(socket, :cur_live, cur_live - 1)
    socket = assign(socket, :rtt, rtt)
    #    Logger.info("Ack: " <> to_string(cur_live))
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

  def adjust_max_live(_active, cur_live, max_live) when cur_live >= max_live do
    Enum.max([max_live - 1, 1])
  end

  def adjust_max_live(_active, cur_live, max_live) when cur_live == 0 do
    Enum.min([max_live + 2, 50])
  end

  def adjust_max_live(_active, _cur_live, max_live) do
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
    resolution = params[:res]
    watch_camera = socket.assigns[:watch_camera]
    hd_cameras = socket.assigns[:hd_cameras]

    camera_active = Map.has_key?(watch_camera, camera_id)
    camera_hd = Map.has_key?(hd_cameras, camera_id)
    frame_active = camera_active and cur_live < max_live
    resolution_active = (resolution == "hd" && camera_hd) or (resolution == "sd" and not camera_hd)

    #    Logger.info(
    #      "liveness: "
    #      <> to_string(cur_live)
    #      <> "/" <> to_string(max_live)
    #      <> to_string(camera_active)
    #      <> " " <> to_string(socket.assigns[:rtt])
    #    )
    new_cur_live =
      if frame_active and resolution_active do
        cur_time = System.monotonic_time(:milliseconds)
        params = Map.put(params, :ts, to_string(cur_time))
        push(socket, "jpg" <> Integer.to_string(camera_id), params)
        cur_live + 1
      else
        cur_live
      end

    socket = assign(socket, :cur_live, new_cur_live)
    {:noreply, socket}
  end

  # Add authorization logic here as required.
  defp authorized?(_payload) do
    true
  end
end
