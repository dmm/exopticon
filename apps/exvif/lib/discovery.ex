defmodule Exvif.Discovery do
  def probe(timeout \\ 1000) do
    message_id = UUID.uuid1()

    request =
      """
      <Envelope xmlns="http://www.w3.org/2003/05/soap-envelope" xmlns:dn="http://www.onvif.org/ver10/network/wsdl">
      <Header>
       <wsa:MessageID xmlns:wsa="http://schemas.xmlsoap.org/ws/2004/08/addressing">#{message_id}</wsa:MessageID>
        <wsa:To xmlns:wsa="http://schemas.xmlsoap.org/ws/2004/08/addressing">urn:schemas-xmlsoap-org:ws:2005:04:discovery</wsa:To>
      	<wsa:Action xmlns:wsa="http://schemas.xmlsoap.org/ws/2004/08/addressing">http://schemas.xmlsoap.org/ws/2005/04/discovery/Probe</wsa:Action>
        </Header>
       <Body>
        <Probe xmlns="http://schemas.xmlsoap.org/ws/2005/04/discovery" xmlns:xsd="http://www.w3.org/2001/XMLSchema" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance">
      	  <Types>dn:NetworkVideoTransmitter</Types>
          <Scopes />
      	</Probe>
      </Body>
      </Envelope>
      """

    {:ok, socket} = :gen_udp.open(3702, [:binary, active: false, multicast_ttl: 2])
    :gen_udp.send(socket, '239.255.255.250', 3702, request)

    cameras = listen(socket, timeout, [])

    :gen_udp.close(socket)

    cameras
  end

  defp listen(socket, timeout, cameras) when timeout > 0 do
    begin_time = :os.system_time(:millisecond)

    cameras =
      :gen_udp.recv(socket, 0, timeout)
      |> handle_message
      |> update_cameras(cameras)

    time_passed = :os.system_time(:millisecond) - begin_time
    listen(socket, timeout - time_passed, cameras)
  end

  defp listen(_, timeout, cameras) when timeout <= 0 do
    cameras
  end

  defp update_cameras({_, nil}, cameras) do
    # When a probe response is invalid or timeout is reached
    # leave cameras unchanged.
    cameras
  end

  defp update_cameras({ip, url}, cameras) do
    [{ip, url} | cameras]
  end

  defp handle_message({:ok, {ip, _, body}}) do
    doc = Exml.parse(body)

    ip =
      Tuple.to_list(ip)
      |> Enum.join(".")

    {ip, Exml.get(doc, "//*[local-name()='XAddrs']")}
  end

  defp handle_message({:error, :timeout}) do
    {nil, nil}
  end
end
