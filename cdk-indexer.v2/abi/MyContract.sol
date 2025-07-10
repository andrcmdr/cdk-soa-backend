// SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;

contract SimpleEvent {
    event ValueSet(address indexed setter, uint256 value);

    function setValue(uint256 v) external {
        emit ValueSet(msg.sender, v);
    }
}
