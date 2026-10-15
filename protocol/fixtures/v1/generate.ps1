# Independent reference fixture builder, no production codec is called.
# Run from any directory with PowerShell. Integer fields are little endian.
$ErrorActionPreference = 'Stop'
$fixtureRoot = $PSScriptRoot
$manifest = [System.Collections.Generic.List[string]]::new()
$manifest.Add('# file|result|type|fields (decimal integers); all IDs are synthetic test values')
$packets = @{}
function Set-Field([byte[]]$Bytes, [int]$Offset, [string]$Type, [string]$Value) {
    $encoded = switch ($Type) {
        'u8' { ,([byte[]]@([byte]::Parse($Value))) }
        'u16' { ,([BitConverter]::GetBytes([uint16]::Parse($Value))) }
        'i16' { ,([BitConverter]::GetBytes([int16]::Parse($Value))) }
        'u32' { ,([BitConverter]::GetBytes([uint32]::Parse($Value))) }
        'u64' { ,([BitConverter]::GetBytes([uint64]::Parse($Value))) }
        default { throw "Unknown integer type $Type" }
    }
    if (-not [BitConverter]::IsLittleEndian) { [Array]::Reverse($encoded) }
    [Array]::Copy($encoded, 0, $Bytes, $Offset, $encoded.Length)
}
function Seal([byte[]]$Bytes) {
    [uint32]$crc = [uint32]::MaxValue
    for ($i = 0; $i -lt 36; $i++) {
        $crc = $crc -bxor $Bytes[$i]
        for ($j = 0; $j -lt 8; $j++) {
            if ($crc -band 1) { $crc = ($crc -shr 1) -bxor [uint32]3988292384 }
            else { $crc = $crc -shr 1 }
        }
    }
    Set-Field $Bytes 36 u32 ([string]($crc -bxor [uint32]::MaxValue))
}
function Valid([string]$Name, [byte]$Tag, [string]$Type, [string]$Fields) {
    $bytes = [byte[]]::new(40)
    $bytes[0] = 86; $bytes[1] = 71; $bytes[2] = 1; $bytes[3] = $Tag
    $values = @()
    foreach ($field in $Fields.Split(' ', [StringSplitOptions]::RemoveEmptyEntries)) {
        $parts = $field.Split(':')
        Set-Field $bytes ([int]$parts[1]) $parts[2] $parts[3]
        $values += "$($parts[0])=$($parts[3])"
    }
    Seal $bytes
    [IO.File]::WriteAllBytes((Join-Path $fixtureRoot "$Name.bin"), $bytes)
    $packets[$Name] = $bytes
    $manifest.Add("$Name.bin|Valid|$Type|$($values -join ' ')")
}
function Invalid([string]$Name, [string]$Base, [string]$Category, [int]$Offset, [byte]$Value, [bool]$Recompute = $true) {
    $bytes = [byte[]]$packets[$Base].Clone()
    $bytes[$Offset] = $Value
    if ($Recompute) { Seal $bytes }
    [IO.File]::WriteAllBytes((Join-Path $fixtureRoot "$Name.bin"), $bytes)
    $manifest.Add("$Name.bin|$Category|$Base|byte[$Offset]=$Value crc_recomputed=$Recompute")
}
Valid hello 1 Hello 'client_nonce:4:u64:72623859790382856 client_instance:12:u64:1230066625199609624 state_rate:20:u16:60 capabilities:22:u16:7 app_build:24:u32:305419896 api_level:28:u16:34'
Valid hello_ack_pending 2 HelloAck 'client_nonce:4:u64:72623859790382856 server_nonce:12:u64:2387509390608836392 session:20:u64:0 state_rate:28:u16:60 timeout_ms:30:u16:250 flags:32:u16:1 capacity:34:u8:1 slot:35:u8:255'
Valid hello_ack_session 2 HelloAck 'client_nonce:4:u64:72623859790382856 server_nonce:12:u64:2387509390608836392 session:20:u64:3544952156018063160 state_rate:28:u16:60 timeout_ms:30:u16:1000 flags:32:u16:14 capacity:34:u8:4 slot:35:u8:3'
Valid pair_request 3 PairRequest 'client_nonce:4:u64:72623859790382856 server_nonce:12:u64:2387509390608836392 client_instance:20:u64:1230066625199609624 code:28:u32:999999 slot:32:u8:255'
Valid pair_accept 4 PairAccept 'session:4:u64:3544952156018063160 client_nonce:12:u64:72623859790382856 server_instance:20:u64:4702394921427289928 state_rate:28:u16:60 timeout_ms:30:u16:100 slot:32:u8:0 capabilities:33:u8:7'
foreach ($reason in 0..6) {
    Valid "pair_reject_$reason" 5 PairReject "client_nonce:4:u64:0 reason:12:u8:$reason retry_after:13:u8:255"
}
$state = 'session:4:u64:3544952156018063160 sequence:12:u32:0 client_ms:16:u32:0 buttons:20:u32:0 left_x:24:i16:0 left_y:26:i16:0 right_x:28:i16:0 right_y:30:i16:0 left_trigger:32:u8:0 right_trigger:33:u8:0'
Valid neutral 16 ControllerState $state
Valid state_one 16 ControllerState ($state.Replace('sequence:12:u32:0', 'sequence:12:u32:1'))
Valid state_half_range 16 ControllerState ($state.Replace('sequence:12:u32:0', 'sequence:12:u32:2147483648'))
Valid state_previous 16 ControllerState ($state.Replace('sequence:12:u32:0', 'sequence:12:u32:4294967295'))
Valid wrong_session 16 ControllerState ($state.Replace('3544952156018063160', '1'))
Valid all_buttons_up_left 16 ControllerState ($state.Replace('buttons:20:u32:0', 'buttons:20:u32:6143'))
Valid all_buttons_down_right 16 ControllerState ($state.Replace('buttons:20:u32:0', 'buttons:20:u32:11263'))
Valid axis_extremes 16 ControllerState ($state.Replace('left_x:24:i16:0', 'left_x:24:i16:-32768').Replace('left_y:26:i16:0', 'left_y:26:i16:32767').Replace('right_x:28:i16:0', 'right_x:28:i16:32767').Replace('right_y:30:i16:0', 'right_y:30:i16:-32768').Replace('right_trigger:33:u8:0', 'right_trigger:33:u8:255'))
Valid mixed 16 ControllerState 'session:4:u64:3544952156018063160 sequence:12:u32:305419896 client_ms:16:u32:2271560481 buttons:20:u32:9253 left_x:24:i16:-12345 left_y:26:i16:23456 right_x:28:i16:13579 right_y:30:i16:-24680 left_trigger:32:u8:85 right_trigger:33:u8:170'
Valid sequence_max 16 ControllerState ($state.Replace('sequence:12:u32:0', 'sequence:12:u32:4294967295').Replace('client_ms:16:u32:0', 'client_ms:16:u32:4294967295').Replace('3544952156018063160', '18446744073709551615'))
Valid sequence_wrap 16 ControllerState ($state.Replace('3544952156018063160', '18446744073709551615'))
Valid status 17 Status 'session:4:u64:3544952156018063160 last_sequence:12:u32:4294967295 client_ms:16:u32:2271560481 accepted:20:u32:4294967295 rejected:24:u32:305419896 lost:28:u16:65535 observed_rate:30:u16:65535 flags:32:u16:63 timeout_ms:34:u16:250'
Valid status_presession 17 Status 'session:4:u64:0 last_sequence:12:u32:0 client_ms:16:u32:0 accepted:20:u32:0 rejected:24:u32:0 lost:28:u16:0 observed_rate:30:u16:0 flags:32:u16:0 timeout_ms:34:u16:250'
foreach ($reason in 0..4) {
    Valid "disconnect_$reason" 18 Disconnect "session:4:u64:3544952156018063160 reason:12:u8:$reason"
}
Invalid bad_magic hello Magic 0 0
Invalid bad_version hello Version 2 2
Invalid bad_type hello MessageType 3 255
Invalid bad_crc hello Crc 24 0 $false
Invalid reserved_hello hello Reserved 30 1
Invalid reserved_request pair_request Reserved 35 1
Invalid reserved_accept pair_accept Reserved 34 1
Invalid reserved_reject pair_reject_0 Reserved 14 1
Invalid reserved_disconnect disconnect_0 Reserved 35 1
Invalid reserved_state neutral Reserved 34 1
Invalid reserved_buttons neutral Reserved 23 128
Invalid reserved_capabilities hello Reserved 23 128
Invalid reserved_server_flags hello_ack_pending Reserved 33 128
Invalid reserved_accepted_capabilities pair_accept Reserved 33 128
Invalid reserved_status_flags status Reserved 33 128
Invalid opposing_vertical all_buttons_up_left DpadConflict 21 31
Invalid opposing_horizontal all_buttons_down_right DpadConflict 21 63
Invalid bad_code pair_request Range 31 1
Invalid bad_slot pair_request Range 32 0
Invalid bad_reason pair_reject_0 Range 12 7
Invalid bad_disconnect_reason disconnect_0 Range 12 5
Invalid bad_capacity hello_ack_pending Range 34 0
Invalid bad_pending_slot hello_ack_pending Range 35 0
Invalid bad_timeout status Range 35 255
foreach ($length in @(0,39,41,1024)) {
    $bytes = [byte[]]::new($length)
    [Array]::Copy($packets['neutral'], $bytes, [Math]::Min($length,40))
    [IO.File]::WriteAllBytes((Join-Path $fixtureRoot "length_$length.bin"), $bytes)
    $manifest.Add("length_$length.bin|Length|neutral|length=$length")
}
[IO.File]::WriteAllLines((Join-Path $fixtureRoot 'manifest.txt'), $manifest)
