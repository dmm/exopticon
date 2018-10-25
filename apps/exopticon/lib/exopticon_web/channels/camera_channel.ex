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

  alias ExopticonWeb.FlowAgent

  require Logger

  intercept(["jpg", "frame"])

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

  def join("camera:" <> room_id, _params, socket) do
    {:ok, socket}
  end

  # Channels can be used in a request/response fashion
  # by sending replies to requests from the client
  def handle_in("ping", payload, socket) do
    {:reply, {:ok, payload}, socket}
  end

  def handle_in("ack", payload, socket) do
    cur_time = System.monotonic_time(:milliseconds)
    %{"ts" => ts} = payload
    {ts_int, _} = Integer.parse(ts)
    new_rtt = cur_time - ts_int

    FlowAgent.ack(socket.id, new_rtt)

    {:noreply, socket}
  end

  # It is also common to receive messages from the client and
  # broadcast to everyone in the current topic (camera:lobby).
  def handle_in("shout", payload, socket) do
    broadcast(socket, "shout", payload)
    {:noreply, socket}
  end

  def handle_out("jpg", params, socket) do
    camera_id = params[:cameraId]
    resolution = params[:res]
    watch_camera = socket.assigns[:watch_camera]
    hd_cameras = socket.assigns[:hd_cameras]

    camera_active = Map.has_key?(watch_camera, camera_id)
    camera_hd = Map.has_key?(hd_cameras, camera_id)
    # cur_live < max_live
    frame_active = camera_active
    resolution_active = (resolution == "hd" && camera_hd) or (resolution == "sd" and not camera_hd)

    #    Logger.info(
    #      "liveness: "
    #      <> to_string(cur_live)
    #      <> "/" <> to_string(max_live)
    #      <> to_string(camera_active)
    #      <> " " <> to_string(socket.assigns[:rtt])
    #    )
    if frame_active and resolution_active and FlowAgent.try_send(socket.id) do
      cur_time = System.monotonic_time(:milliseconds)
      params = Map.put(params, :ts, to_string(cur_time))
      push(socket, "jpg" <> Integer.to_string(camera_id), params)
    end

    {:noreply, socket}
  end

  def handle_out("frame", params, socket) do
    if true do #FlowAgent.try_send(socket.id) do
      cur_time = System.monotonic_time(:milliseconds)
      params = Map.put(params, :ts, to_string(cur_time))
      push(socket, "frame", params)
    end
    {:noreply, socket}
  end

  def terminate(_reason, socket) do
    IO.puts("RESETTING: " <> socket.id)
    FlowAgent.reset(socket.id)
  end

  # Add authorization logic here as required.
  defp authorized?(_payload) do
    true
  end
end
