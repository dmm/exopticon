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

require Logger

defmodule Exopticon.Video.FrameCaptureServer do
  @moduledoc """
  Provides server that deletes video files as needed
  """
  use GenServer

  require Logger

  alias Exopticon.Video

  def start_link do
    GenServer.start_link(__MODULE__, [], name: __MODULE__)
  end

  def init(_) do
    schedule_work()
    {:ok, []}
  end

  def handle_info(:work, []) do
    # Not working, let's go!
    annotations = Video.list_unframed_snapshots()
    work(annotations)
    schedule_work()
    {:noreply, []}
  end

  def work([a | rest]) do
    path =
      a.video_unit.camera.camera_group.storage_path
      |> Path.join(Integer.to_string(a.video_unit.camera.id))
      |> Path.join('/annotations')

    File.mkdir(path)
    hd_filename = Path.join(path, "#{a.id}.hd.jpg")
    sd_filename = Path.join(path, "#{a.id}.sd.jpg")
    [file | _] = a.video_unit.files

    Logger.info(
      "Generating frames for annotation #{a.id} #{file.filename} #{sd_filename} #{hd_filename}"
    )

    {ret, _} = generate_frames(file.filename, a.frame_index, sd_filename, hd_filename)

    if ret == :ok and File.exists?(sd_filename) do
      a
      |> Exopticon.Video.update_annotation(%{hd_filename: hd_filename, sd_filename: sd_filename})
    end

    if ret != :ok do
      Logger.error("Error generating frame for #{file.filename}")
    end

    work(rest)
  end

  def work([]) do
    # all done!
    {:ok, []}
  end

  def generate_frames(input_file, frame_index, sd_filename, hd_filename) do
    {output, ret} =
      System.cmd(
        "ffmpeg",
        [
          "-y",
          "-i",
          input_file,
          "-filter_complex",
          "select='gte(n,#{frame_index})'[fout];[fout]split=2[in1][in2];[in1]null[hd];[in2]scale=480:-1[sd]",
          "-map",
          "[hd]",
          "-frames:v",
          "1",
          "-qscale:v",
          "2",
          hd_filename,
          "-map",
          "[sd]",
          "-frames:v",
          "1",
          "-qscale:v",
          "2",
          sd_filename
        ],
        parallelism: true,
        stderr_to_stdout: true
      )

    Logger.debug(fn -> "Generating frames for #{input_file}" end)
    Logger.debug(output)

    if ret == 0 do
      {:ok, 0}
    else
      Logger.error("Generating frame for #{input_file} failed, code #{Integer.to_string(ret)}")
      Logger.error(output)
      {:error, ret}
    end
  end

  defp schedule_work do
    Process.send_after(self(), :work, 1000)
  end
end
