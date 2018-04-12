# This file is part of Exopticon (https://github.com/dmm/exopticon).
# Copyright (c) 2018 David Matthew Mattli
#
# Exopticon is free software: you can redistribute it and/or modify
# it under the terms of the GNU General Public License as published by
# the Free Software Foundation, either version 3 of the License, or
# (at your option) any later version.
#
# Exopticon is distributed in the hope that it will be useful,
# but WITHOUT ANY WARRANTY; without even the implied warranty of
# MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
# GNU General Public License for more details.
#
# You should have received a copy of the GNU General Public License
# along with Exopticon.  If not, see <http://www.gnu.org/licenses/>.

defmodule Exopticon.FileLibrary do
  @moduledoc """
  Provides datastructure which allows for the organization of a sequence of video files.
  """
  # microseconds
  @max_gap 500 * 1000

  defp earliest(date1, date2) do
    if Timex.before?(date1, date2) do
      date1
    else
      date2
    end
  end

  defp latest(date1, date2) do
    if Timex.after?(date1, date2) do
      date1
    else
      date2
    end
  end

  defp generate_beginning_gap(_, []) do
    []
  end

  defp generate_beginning_gap(begin_time, [first | _]) do
    gap = Timex.diff(first.begin_time, begin_time, :microseconds)

    case gap > @max_gap do
      true ->
        [
          %{
            begin_time: begin_time,
            end_time: Timex.shift(first.begin_time, microseconds: -1),
            type: :no_video
          }
        ]

      false ->
        []
    end
  end

  defp generate_end_gap(_, nil) do
    []
  end

  defp generate_end_gap(end_time, last_file) do
    gap = Timex.diff(end_time, last_file.end_time, :microseconds)

    case gap > @max_gap do
      true ->
        [
          %{
            begin_time: Timex.shift(last_file.end_time, microseconds: 1),
            end_time: end_time,
            type: :no_video
          }
        ]

      false ->
        []
    end
  end

  defp append_file([], file) do
    [
      %{
        begin_time: file.begin_time,
        end_time: file.end_time,
        type: :video
      }
    ]
  end

  defp append_file(chunks, file) do
    [last | rest] = chunks
    gap = Timex.diff(file.begin_time, last.end_time, :microseconds)

    new_chunks =
      case gap < @max_gap do
        true ->
          [
            %{
              begin_time: last.begin_time,
              end_time: file.end_time,
              type: :video
            }
          ]

        false ->
          [
            %{
              begin_time: file.begin_time,
              end_time: file.end_time,
              type: :video
            },
            %{
              begin_time: Timex.shift(last.end_time, microseconds: 1),
              end_time: Timex.shift(file.begin_time, microseconds: -1),
              type: :no_video
            },
            last
          ]
      end

    new_chunks ++ rest
  end

  defp calculate_availability(begin_time, end_time, [], []) do
    # handle case with no video
    chunks = [
      %{
        begin_time: begin_time,
        end_time: end_time,
        type: :no_video
      }
    ]

    calculate_availability(begin_time, end_time, chunks, [])
  end

  defp calculate_availability(_begin_time, _end_time, chunks, []) do
    chunks |> Enum.reverse()
  end

  defp calculate_availability(begin_time, _end_time, chunks, files) do
    [next_file | rest] = files

    begin_time = earliest(begin_time, next_file.begin_time)
    end_time = latest(begin_time, next_file.end_time)
    new_chunks = append_file(chunks, next_file)

    calculate_availability(next_file.end_time, end_time, new_chunks, rest)
  end

  def calculate_availability(files, begin_time, end_time) do
    gap = generate_beginning_gap(begin_time, files)
    end_gap = generate_end_gap(end_time, List.last(files))

    %{
      begin_time: begin_time,
      end_time: end_time,
      availability: gap ++ calculate_availability(begin_time, end_time, [], files) ++ end_gap
    }
  end
end
