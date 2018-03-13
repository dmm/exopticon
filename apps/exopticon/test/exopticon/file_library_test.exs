defmodule Exopticon.FileLibraryTest do
  use ExUnit.Case, async: true
  use Timex

  alias Exopticon.FileLibrary
  alias Exopticon.Video.File

  def check_order(_, false) do
    false
  end

  def check_order([], true) do
    true
  end

  def check_order([first | []], cont) do
    gap = Timex.diff(first.end_time, first.begin_time, :microseconds)

    gap > 0 and cont
  end

  def check_order([first, second | rest], _) do

    gap = Timex.diff(first.end_time, first.begin_time, :microseconds)
    file_gap = Timex.diff(second.begin_time, first.end_time, :microseconds)

    check_order(rest, gap > 0 and file_gap > 0)
  end

  test "empty file list results in empty availability" do
    begin_time = Timex.parse!("2016-02-29T12:30:30.120+00:00", "{ISO:Extended}")
    end_time = Timex.shift(begin_time, minutes: 3)

    chunks = FileLibrary.calculate_availability([], begin_time, end_time)

    assert DateTime.diff(chunks.begin_time, begin_time) == 0
    assert DateTime.diff(chunks.end_time, end_time) == 0

    assert length(chunks.availability) == 1

    [ head | _tail ] = chunks.availability
    assert head.type == :no_video
    assert DateTime.diff(head.begin_time, begin_time) == 0
    assert DateTime.diff(head.end_time, end_time) == 0
  end

  test "single file results in single availability chunk" do
    begin_time = Timex.parse!("2016-02-29T12:30:30.120+00:00", "{ISO:Extended}")
    end_time = Timex.shift(begin_time, minutes: 3)
    files = [%File{begin_time: begin_time, end_time: end_time}]

    chunks = FileLibrary.calculate_availability(files, begin_time, end_time)

    assert length(chunks.availability) == 1

    ch = List.first(chunks.availability)
    assert ch.begin_time == begin_time
    assert ch.end_time == end_time
    assert ch.type == :video
  end

  test "two continuous files result in single video chunk" do
    begin_time = Timex.parse!("2016-02-29T12:30:30.120+00:00", "{ISO:Extended}")
    end_time = Timex.shift(begin_time, minutes: 3)
    files = [
      %File{
        begin_time: begin_time,
        end_time: end_time
      },
      %File{
        begin_time: Timex.shift(end_time, microseconds: 5 * 1000),
        end_time: Timex.shift(end_time, minutes: 5)
      }
    ]

    chunks = FileLibrary.calculate_availability(files, begin_time, end_time)

    assert length(chunks.availability) == 1

    c = List.first(chunks.availability)
    assert c.begin_time == begin_time
    assert c.end_time == Timex.shift(end_time, minutes: 5)
    assert c.type == :video

    assert check_order(chunks.availability, true)
  end

  test "two discontiguous files results in three chunks" do
    begin_time = Timex.parse!("2016-02-29T12:30:30.120+00:00", "{ISO:Extended}")
    end_time = Timex.shift(begin_time, minutes: 3)
    files = [
      %File{
        begin_time: begin_time,
        end_time: end_time
      },
      %File{
        begin_time: Timex.shift(end_time, seconds: 30),
        end_time: Timex.shift(end_time, minutes: 5)
      }
    ]

    chunks = FileLibrary.calculate_availability(files, begin_time, end_time)

    assert length(chunks.availability) == 3

    assert check_order(chunks.availability, true), "not in order!"
  end
end
