defmodule Exopticon.EqualQueueTest do
  use ExUnit.Case, async: true

  alias Exopticon.EqualQueue

  test "empty queue has len == 0" do
    q = EqualQueue.new()

    assert EqualQueue.len(q) == 0
  end

  test "push item increases len" do
    q = EqualQueue.new()

    q2 = EqualQueue.push(q, 2, "testesttest")

    assert EqualQueue.len(q2) == 1
  end

  test "popping returns same item as pushed" do
    q = EqualQueue.new()

    val = "test test test"
    q2 = EqualQueue.push(q, 2, val)

    {{_, val2}, _} = EqualQueue.pop(q2)
    assert val == val2
  end

  test "popping returns first item pushed before second item" do
    q = EqualQueue.new()

    val1 = "test"
    val2 = "qwer"
    q2 = EqualQueue.push(q, 3, val1)
    q3 = EqualQueue.push(q2, 4, val2)

    {{_, val3}, _} = EqualQueue.pop(q3)

    assert val3 == val1
  end
end
