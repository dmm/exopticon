defmodule Exvif.Cam do
  import Exvif.Util

  def request_date_and_time(url) do
    body = """
    <s:Envelope xmlns:s="http://www.w3.org/2003/05/soap-envelope">
    <s:Body xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance" xmlns:xsd="http://www.w3.org/2001/XMLSchema">
    <GetSystemDateAndTime xmlns="http://www.onvif.org/ver10/device/wsdl"/>
    </s:Body>
      </s:Envelope>
    """

    request(url, body)
  end

  def handle_date_and_time({:ok, %{status_code: 200, body: body}}) do
    doc = Exml.parse(body)

    [time, date] = Exml.get(doc, "//tt:UTCDateTime")

    time = Enum.map(time, fn x -> Integer.parse(x) |> elem(0) end)
    date = Enum.map(date, fn x -> Integer.parse(x) |> elem(0) end)

    [hour, minute, sec] = time
    [year, month, day] = date

    {:ok, Timex.to_datetime({List.to_tuple(date), List.to_tuple(time)}, "Etc/UTC")}
  end

  def fetch_date_and_time(url) do
    request_date_and_time(url)
    |> handle_date_and_time
  end

  def request_capabilities(url, username, password) do
    body =
      envelope_header(username, password) <>
        """
        <GetCapabilities xmlns="http://www.onvif.org/ver10/device/wsdl">
          <Category>All</Category>
        </GetCapabilities>'
        """ <> envelope_footer()

    request(url, body)
  end

  def handle_capabilities({:ok, %{status_code: 200, body: body}}) do
    doc = Exml.parse(body)

    doc
  end

  def fetch_capabilities(url, username, password) do
    request_capabilities(url, username, password)
    |> handle_capabilities
  end

  def request_network_interfaces(url, username, password) do
    body =
      envelope_header(username, password) <>
        """
        <GetNetworkInterfaces xmlns="http://www.onvif.org/ver10/device/wsdl"/>
        """ <> envelope_footer()

    request(url, body)
  end

  def parse_network_interfaces(doc, token) do
    interface_selector = "//tds:NetworkInterfaces[@token='#{token}']"
    sel = fn selector -> Exml.get(doc, interface_selector <> selector) end

    %{
      token: token,
      name: sel.("//tt:Name"),
      address: sel.("//tt:IPv4//tt:Address") |> List.wrap() |> List.first(),
      mac: sel.("//tt:HwAddress")
           |> String.upcase()
           |> String.replace(~r/[\:\-\.]/, "")
           |> String.to_charlist()
           |> Enum.chunk(2)
           |> Enum.join("-"),
      dhcp: sel.("//tt:DHCP")
    }
  end

  def handle_network_interfaces({:ok, %{status_code: 200, body: body}}) do
    doc = Exml.parse(body)

    interface_tokens = List.wrap(Exml.get(doc, "//tds:NetworkInterfaces/@token"))
    Enum.map(interface_tokens, fn x -> parse_network_interfaces(doc, x) end)
  end

  def fetch_network_interfaces(url, username, password) do
    request_network_interfaces(url, username, password)
    |> handle_network_interfaces
  end

  def request_profiles(url, username, password) do
    body =
      envelope_header(username, password) <>
        """
        <GetProfiles xmlns="http://www.onvif.org/ver10/media/wsdl"/>
        """ <> envelope_footer()

    request(url, body)
  end

  def to_int(number_string) when is_binary(number_string) do
    case Integer.parse(number_string) do
      {:error, reason} -> 0
      {number, _} -> number
    end
  end

  def to_int(number) when is_integer(number) do
    number
  end

  def to_int(nil) do
    0
  end

  def parse_profile(doc, token) do
    profile_selector = "//trt:Profiles[@token='#{token}']"
    sel = fn selector -> Exml.get(doc, profile_selector <> selector) end

    %{
      name: sel.("/tt:Name"),
      profile_token: token,
      video_source_configuration: %{
        name: sel.("/tt:VideoSourceConfiguration/tt:Name"),
        use_count: to_int(sel.("/tt:VideoSourceConfiguration/tt:UseCount")),
        source_token: sel.("/tt:VideoSourceConfiguration/tt:SourceToken"),
        bounds: "TODO"
      },
      video_encoder_configuration: %{
        name: sel.("/tt:VideoEncoderConfiguration/tt:Name"),
        use_count: to_int(sel.("/tt:VideoEncoderConfiguration/tt:UseCount")),
        encoding: sel.("/tt:VideoEncoderConfiguration/tt:Encoding"),
        resolution: %{
          width: to_int(sel.("/tt:VideoEncoderConfiguration/tt:Resolution/tt:Width")),
          height: to_int(sel.("/tt:VideoEncoderConfiguration/tt:Resolution/tt:Height"))
        },
        quality: to_int(sel.("/tt:VideoEncoderConfiguration/tt:Quality")),
        rate_control: %{
          frame_rate_limit: to_int(
            sel.("/tt:VideoEncoderConfiguration/tt:RateControl/tt:FrameRateLimit")
          ),
          encoding_interval: to_int(
            sel.("/tt:VideoEncoderConfiguration/tt:RateControl/tt:EncodingInterval")
          ),
          bitrate_limit: to_int(
            sel.("/tt:VideoEncoderConfiguration/tt:RateControl/tt:BitrateLimit")
          )
        }
      }
    }
  end

  def handle_profiles({:ok, %{status_code: 200, body: body}}) do
    doc = Exml.parse(body)

    profile_tokens = Exml.get(doc, "//trt:Profiles/@token")
    Enum.map(profile_tokens, fn x -> parse_profile(doc, x) end)
  end

  def fetch_profiles(url, username, password) do
    request_profiles(url, username, password)
    |> handle_profiles
  end

  def request_device_information(url, username, password) do
    body =
      envelope_header(username, password) <>
        """
        <GetDeviceInformation xmlns="http://www.onvif.org/ver10/device/wsdl"/>
        """ <> envelope_footer()

    request(url, body)
  end

  def handle_device_information({:ok, %{status_code: 200, body: body}}) do
    doc = Exml.parse(body)

    %{
      manufacturer: Exml.get(doc, "//tds:Manufacturer"),
      model: Exml.get(doc, "//tds:Model"),
      firmware_verison: Exml.get(doc, "//tds:FirmwareVersion"),
      serial_number: Exml.get(doc, "//tds:SerialNumber"),
      hardware_id: Exml.get(doc, "//tds:HardwareId")
    }
  end

  def fetch_device_information(url, username, password) do
    request_device_information(url, username, password)
    |> handle_device_information
  end

  def request_stream_uri(url, username, password, profile_token) do
    stream_type = 'RTP-Unicast'

    body =
      envelope_header(username, password) <>
        """
        <GetStreamUri xmlns="http://www.onvif.org/ver10/media/wsdl">
        <StreamSetup>
         <Stream xmlns="http://www.onvif.org/ver10/schema">#{stream_type}</Stream>
        <Transport xmlns="http://www.onvif.org/ver10/schema">
        	 <Protocol>RTSP</Protocol>
         </Transport>
        </StreamSetup>
        <ProfileToken>#{profile_token}</ProfileToken>
        </GetStreamUri>
        """ <> envelope_footer()

    request(url, body)
  end

  def handle_stream_uri({:ok, %{status_code: 200, body: body}}, username, password) do
    doc = Exml.parse(body)

    uri = Exml.get(doc, "//tt:Uri")

    cond do
      String.match?(uri, ~r/:\/\/.*:.*@.*/) -> uri
      true -> Regex.replace(~r/:\/\//, uri, "://#{username}:#{password}@")
    end
  end

  def fetch_stream_uri(url, username, password, profile_token) do
    request_stream_uri(url, username, password, profile_token)
    |> handle_stream_uri(username, password)
  end

  def request_ptz_configurations(url, username, password) do
    body =
      envelope_header(username, password) <>
        """
        <GetConfigurations xmlns="http://www.onvif.org/ver20/ptz/wsdl">
        </GetConfigurations>
        """ <> envelope_footer()

    request(url, body)
  end

  def ptz_vectors(x, y) do
    """
    <PanTilt x="#{x}" y="#{y}" xmlns="http://www.onvif.org/ver10/schema"/>
    """
  end

  def request_ptz_relative_move(url, username, password, profile_token, x, y) do
    vectors = ptz_vectors(x, y)

    body =
      envelope_header(username, password) <>
        """
        <RelativeMove xmlns="http://www.onvif.org/ver20/ptz/wsdl">
        <ProfileToken>#{profile_token}</ProfileToken>
        <Translation>
            #{vectors}
        </Translation>
        </RelativeMove>
        """ <> envelope_footer()

    request(url, body)
  end

  def fetch_camera(url, username, password) do
    profiles = fetch_profiles(url, username, password)

    uris =
      Enum.into(
        Enum.map(profiles, fn p ->
          {
            String.to_atom(p[:profile_token]),
            fetch_stream_uri(url, username, password, p[:profile_token])
          }
        end),
        %{}
      )

    device_information = fetch_device_information(url, username, password)
    network_interfaces = fetch_network_interfaces(url, username, password)

    Map.merge(device_information, %{
      profiles: profiles,
      stream_uris: uris,
      interface: network_interfaces
    })
  end

  def fetch_camera(hostname, port, username, password) do
    cam_url(hostname, port)
    |> fetch_camera(username, password)
  end

  def cam_url(hostname, port) when is_integer(port) do
    cam_url(hostname, Integer.to_string(port))
  end

  def cam_url(hostname, port) when is_binary(port) do
    "http://#{hostname}:#{port}/onvif/device_service"
  end
end
