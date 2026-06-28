#!/bin/bash
export DEBIAN_FRONTEND=noninteractive

# Colors for terminal output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
MAGENTA='\033[0;35m'
NC='\033[0m' # No Color

SERVICE_NAME="ToRouter"
SERVICE_FILE="/opt/ToRouter-Multi-Location/dist/ToRouter.service"
SERVICE_DEST="/etc/systemd/system/ToRouter.service"
APP_DIR="/opt/ToRouter-Multi-Location"

# Function to print colored commands
print_commands() {
    echo ""
    echo -e "${CYAN}════════════════════════════════════════════════════════════════${NC}"
    echo -e "${GREEN}▶ To restart the service:${NC}"
    echo -e "  ${YELLOW}sudo systemctl restart ${SERVICE_NAME}.service${NC}"
    echo ""
    echo -e "${BLUE}▶ To check service status:${NC}"
    echo -e "  ${YELLOW}sudo systemctl status ${SERVICE_NAME}.service${NC}"
    echo ""
    echo -e "${GREEN}▶ To view real-time logs:${NC}"
    echo -e "  ${YELLOW}sudo journalctl -u ${SERVICE_NAME}.service -f${NC}"
}

# Function to print colored output
print_colored() {
    local color=$1
    local message=$2
    echo -e "${color}${message}${NC}"
}

# Function to check if service is running
is_service_active() {
    systemctl is-active ${SERVICE_NAME}.service
    return $?
}

# Function to check if service file exists
check_service_file() {
    if [ ! -f "$SERVICE_FILE" ]; then
        print_colored "$RED" "✗ Error: Service file not found at $SERVICE_FILE"
        exit 1
    fi
}

# Function to start the service
start_service() {
    clear
    print_colored "$CYAN" "╔═══════════════════════════════════════════════════════════════╗"
    print_colored "$CYAN" "║          🚀 Starting ToRouter Service                       ║"
    print_colored "$CYAN" "╚═══════════════════════════════════════════════════════════════╝"
    echo ""
    
    # Install required packages
    print_colored "$YELLOW" "📦 Installing required packages (libssl-dev, libevent-dev)..."
    sudo apt-get update
    sudo apt-get install -y libssl-dev libevent-dev
    if [ $? -ne 0 ]; then
        print_colored "$RED" "✗ Error: Failed to install required packages"
        exit 1
    fi
    print_colored "$GREEN" "✓ Required packages installed successfully"
    echo ""
    
    # Check if service file exists
    check_service_file
    
    # Copy service file
    print_colored "$YELLOW" "📁 Copying service file to /etc/systemd/system/..."
    sudo cp "$SERVICE_FILE" "$SERVICE_DEST"
    if [ $? -ne 0 ]; then
        print_colored "$RED" "✗ Error: Failed to copy service file"
        exit 1
    fi
    print_colored "$GREEN" "✓ Service file copied successfully"
    
    # Reload systemd
    print_colored "$YELLOW" "🔄 Reloading systemd..."
    sudo systemctl daemon-reload
    print_colored "$GREEN" "✓ Systemd reloaded successfully"
    
    # Enable service
    print_colored "$YELLOW" "🔗 Enabling service (auto-start on boot)..."
    sudo systemctl enable ${SERVICE_NAME}.service
    print_colored "$GREEN" "✓ Service enabled successfully"
    
    # Start service
    print_colored "$YELLOW" "▶ Starting service..."
    sudo systemctl start ${SERVICE_NAME}.service
    if [ $? -ne 0 ]; then
        print_colored "$RED" "✗ Error: Failed to start service"
        print_colored "$YELLOW" "ℹ Check logs for more details: sudo journalctl -u ${SERVICE_NAME}.service -n 20"
        exit 1
    fi
    print_colored "$GREEN" "✓ Service started successfully"
    
    # Show status
    echo ""
    print_colored "$CYAN" "╔═══════════════════════════════════════════════════════════════╗"
    print_colored "$CYAN" "║          📊 Service Status                                  ║"
    print_colored "$CYAN" "╚═══════════════════════════════════════════════════════════════╝"
    sudo systemctl status ${SERVICE_NAME}.service --no-pager
    
    # Show useful commands
    print_commands
    
    print_colored "$GREEN" "\n✅ Service installation and startup completed successfully!"
}

# Function to stop and remove service
stop_service() {
    clear
    print_colored "$CYAN" "╔═══════════════════════════════════════════════════════════════╗"
    print_colored "$CYAN" "║          ⏹ Stopping ToRouter Service                        ║"
    print_colored "$CYAN" "╚═══════════════════════════════════════════════════════════════╝"
    echo ""
    
    # Check if service exists
    if [ -f "$SERVICE_DEST" ]; then
        # Check if service is running
        if is_service_active; then
            print_colored "$YELLOW" "⏹ Stopping service..."
            sudo systemctl stop ${SERVICE_NAME}.service
            print_colored "$GREEN" "✓ Service stopped successfully"
        else
            print_colored "$YELLOW" "ℹ Service is not running"
        fi
        
        # Disable service
        print_colored "$YELLOW" "🔗 Disabling service..."
        sudo systemctl disable ${SERVICE_NAME}.service
        print_colored "$GREEN" "✓ Service disabled successfully"
        
        # Remove service file
        print_colored "$YELLOW" "🗑 Removing service file..."
        sudo rm -f "$SERVICE_DEST"
        print_colored "$GREEN" "✓ Service file removed successfully"
        
        # Reload systemd
        print_colored "$YELLOW" "🔄 Reloading systemd..."
        sudo systemctl daemon-reload
        print_colored "$GREEN" "✓ Systemd reloaded successfully"
        
        print_colored "$GREEN" "\n✅ Service stopped and removed successfully!"
    else
        print_colored "$YELLOW" "⚠️  Service file not found. Already removed or never installed."
    fi
}

# Function to uninstall completely
uninstall_service() {
    clear
    print_colored "$CYAN" "╔═══════════════════════════════════════════════════════════════╗"
    print_colored "$CYAN" "║          🗑 Uninstalling ToRouter Completely                 ║"
    print_colored "$CYAN" "╚═══════════════════════════════════════════════════════════════╝"
    echo ""
    
    # Stop and remove service
    stop_service
    
    # Remove application directory
    echo ""
    print_colored "$YELLOW" "🗑 Removing application directory..."
    if [ -d "$APP_DIR" ]; then
        sudo rm -rf "$APP_DIR"
        print_colored "$GREEN" "✓ Application directory removed successfully!"
    else
        print_colored "$YELLOW" "⚠️  Application directory not found at $APP_DIR"
    fi
    
    print_colored "$GREEN" "\n✅ Uninstallation completed successfully!"
}

# Function to show usage
show_usage() {
    echo -e "${CYAN}╔═══════════════════════════════════════════════════════════════╗${NC}"
    echo -e "${CYAN}║          📦 ToRouter Installation Manager                    ║${NC}"
    echo -e "${CYAN}╚═══════════════════════════════════════════════════════════════╝${NC}"
    echo ""
    echo -e "${GREEN}Usage:${NC} $0 {${YELLOW}start${NC}|${YELLOW}stop${NC}|${YELLOW}uninstall${NC}}"
    echo ""
    echo -e "${BLUE}Commands:${NC}"
    echo -e "  ${GREEN}start${NC}     - Install and start the ToRouter service"
    echo -e "  ${YELLOW}stop${NC}      - Stop and remove the ToRouter service"
    echo -e "  ${RED}uninstall${NC}  - Stop service and completely remove application"
    echo ""
    echo -e "${MAGENTA}Example:${NC}"
    echo -e "  ${YELLOW}sudo $0 start${NC}"
    echo ""
}

# Main script logic
case "$1" in
    start)
        start_service
        ;;
    stop)
        stop_service
        ;;
    uninstall)
        uninstall_service
        ;;
    *)
        show_usage
        exit 1
        ;;
esac

exit 0
